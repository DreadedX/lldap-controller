use std::collections::BTreeMap;
use std::str::from_utf8;
use std::sync::Arc;
use std::time::Duration;

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
use tracing::{debug, instrument, trace};

use crate::context::{Context, ControllerEvents};
use crate::lldap;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Failed to commit: {0}")]
    Commit(#[from] kube::api::entry::CommitError),
    #[error("Kube api error: {0}")]
    Kube(#[from] kube::Error),
    #[error("LLDAP error: {0}")]
    Lldap(#[from] lldap::Error),
    #[error("MissingObjectKey: {0}")]
    MissingObjectKey(&'static str),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

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

impl ServiceUser {
    #[instrument(skip(self, ctx))]
    pub async fn reconcile(self: Arc<Self>, ctx: Arc<Context>) -> Result<Action> {
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

        debug!(name, "reconcile request");

        let secret_name = format!("{name}-lldap-credentials");
        let username = format!("{name}.{namespace}");

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
        if lldap_client.list_users().await?.any(|id| id == username) {
            debug!(name, username, "User already exists");
        } else {
            debug!(name, username, "Creating new user");

            lldap_client.create_user(&username).await?;
            ctx.recorder.user_created(self.as_ref(), &username).await?;
        }

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
}

#[cfg(test)]
mod tests {
    use super::*;
    use kube::CustomResourceExt;

    #[test]
    fn service_user_crd_output() {
        insta::assert_yaml_snapshot!(ServiceUser::crd());
    }
}
