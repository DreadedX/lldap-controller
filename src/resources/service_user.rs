use std::collections::BTreeMap;
use std::str::from_utf8;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use k8s_openapi::api::core::v1::Secret;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::OwnerReference;
use kube::api::{ObjectMeta, Patch, PatchParams, PostParams};
use kube::runtime::controller::Action;
use kube::{Api, CustomResource, Resource};
use passwords::PasswordGenerator;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::{debug, trace, warn};

use super::{Error, Reconcile, Result};
use crate::context::{Context, ControllerEvents};
use crate::lldap;

#[derive(CustomResource, Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[kube(
    kind = "ServiceUser",
    group = "lldap.huizinga.dev",
    version = "v1",
    namespaced,
    status = "ServiceUserStatus"
)]
#[kube(
    shortname = "lsu",
    doc = "Custom resource for managing Service Users inside of LLDAP",
    printcolumn = r#"{"name":"Manager", "type":"boolean", "description":"Can the service user manage passwords", "jsonPath":".spec.passwordManager"}"#,
    printcolumn = r#"{"name":"Password", "type":"date", "description":"Secret creation timestamp", "jsonPath":".status.secretCreated"}"#,
    printcolumn = r#"{"name":"Age", "type":"date", "jsonPath":".metadata.creationTimestamp"}"#
)]
#[serde(rename_all = "camelCase")]
pub struct ServiceUserSpec {
    #[serde(default)]
    password_manager: bool,
    #[serde(default)]
    additional_groups: Vec<String>,
}

#[derive(Deserialize, Serialize, Clone, Default, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ServiceUserStatus {
    pub secret_created: Option<DateTime<Utc>>,
}

fn new_secret(username: &str, oref: OwnerReference) -> Secret {
    let pg = PasswordGenerator::new()
        .length(32)
        .uppercase_letters(true)
        .strict(true);

    let mut contents = BTreeMap::new();
    contents.insert("username".into(), username.into());
    contents.insert(
        "password".into(),
        pg.generate_one().expect("Settings should be valid"),
    );

    Secret {
        metadata: ObjectMeta {
            owner_references: Some(vec![oref]),
            ..Default::default()
        },
        string_data: Some(contents),
        ..Default::default()
    }
}

fn format_username(name: &str, namespace: &str) -> String {
    format!("{name}.{namespace}")
}

#[async_trait]
impl Reconcile for ServiceUser {
    async fn reconcile(self: Arc<Self>, ctx: Arc<Context>) -> Result<Action> {
        let name = self
            .metadata
            .name
            .clone()
            .ok_or(Error::MissingObjectKey(".metadata.name"))?;
        let namespace = self
            .metadata
            .namespace
            .clone()
            .ok_or(Error::MissingObjectKey(".metadata.namespace"))?;
        let oref = self
            .controller_owner_ref(&())
            .expect("Field should populated by apiserver");

        debug!(name, "Apply");

        let secret_name = format!("{name}-lldap-credentials");
        let username = format_username(&name, &namespace);

        let client = &ctx.client;
        let secrets = Api::<Secret>::namespaced(client.clone(), &namespace);

        // TODO: Potentially issue: someone modifies the secret and removes the pass
        trace!(name, "Get or create secret");
        let mut created = false;
        let mut secret = secrets
            .entry(&secret_name)
            .await?
            .and_modify(|_| {
                debug!(name, secret_name, "Secret already exists");
            })
            .or_insert(|| {
                created = true;
                debug!(name, secret_name, "Generating new secret");

                new_secret(&username, oref)
            });

        trace!(name, "Committing secret");
        secret
            .commit(&PostParams {
                dry_run: false,
                field_manager: Some(ctx.controller_name.clone()),
            })
            .await?;
        let secret = secret;

        if created {
            trace!(name, "Sending secret creating notification");
            // The reason this is here instead of inside the or_insert is that we
            // want to send the event _after_ it successfully committed.
            // Also or_insert is not async!
            ctx.recorder
                .secret_created(self.as_ref(), secret.get())
                .await?;
        }

        let lldap_client = ctx.lldap_config.build_client().await?;

        trace!(name, "Creating user if needed");
        let user = match lldap_client.get_user(&username).await {
            Err(lldap::Error::GraphQl(err))
                if err.message == format!("Entity not found: `{username}`") =>
            {
                debug!(name, username, "Creating new user");

                let user = lldap_client.create_user(&username).await?;
                ctx.recorder.user_created(self.as_ref(), &username).await?;

                Ok(user)
            }
            Ok(user) => {
                debug!(name, username, "User already exists");

                Ok(user)
            }
            Err(err) => Err(err),
        }?;

        trace!(name, "Updating groups");
        let mut groups = self.spec.additional_groups.clone();
        groups.push(
            if self.spec.password_manager {
                "lldap_password_manager"
            } else {
                "lldap_strict_readonly"
            }
            .to_owned(),
        );
        lldap_client.update_user_groups(&user, &groups).await?;

        trace!(name, "Updating password");
        let password = secret.get().data.as_ref().unwrap().get("password").unwrap();
        let password = from_utf8(&password.0).unwrap();
        lldap_client.update_password(&username, password).await?;

        trace!(name, "Updating status");
        let service_users = Api::<ServiceUser>::namespaced(client.clone(), &namespace);
        let status = json!({
            "status": ServiceUserStatus { secret_created: secret.get().meta().creation_timestamp.as_ref().map(|ts| ts.0) }
        });
        service_users
            .patch_status(&name, &PatchParams::default(), &Patch::Merge(&status))
            .await?;

        Ok(Action::requeue(Duration::from_secs(3600)))
    }

    async fn cleanup(self: Arc<Self>, ctx: Arc<Context>) -> Result<Action> {
        let name = self
            .metadata
            .name
            .clone()
            .ok_or(Error::MissingObjectKey(".metadata.name"))?;
        let namespace = self
            .metadata
            .namespace
            .clone()
            .ok_or(Error::MissingObjectKey(".metadata.namespace"))?;

        debug!(name, "Cleanup");

        let username = format_username(&name, &namespace);

        let lldap_client = ctx.lldap_config.build_client().await?;

        trace!(name, username, "Deleting user");
        match lldap_client.delete_user(&username).await {
            Err(lldap::Error::GraphQl(err))
                if err.message == format!("Entity not found: `No such user: '{username}'`") =>
            {
                ctx.recorder
                    .user_not_found(self.as_ref(), &username)
                    .await?;
                warn!(name, username, "User not found");
                Ok(())
            }
            Ok(_) => {
                ctx.recorder.user_deleted(self.as_ref(), &username).await?;
                Ok(())
            }
            Err(err) => Err(err),
        }?;

        Ok(Action::await_change())
    }
}

#[cfg(test)]
mod tests {
    use kube::CustomResourceExt;

    use super::*;

    #[test]
    fn service_user_crd_output() {
        insta::assert_yaml_snapshot!(ServiceUser::crd());
    }
}
