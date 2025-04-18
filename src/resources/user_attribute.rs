use std::time::Duration;

use kube::api::{Patch, PatchParams};
use kube::runtime::controller::Action;
use kube::{Api, CELSchema, CustomResource};
use queries::AttributeType;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::{debug, trace, warn};

use super::Reconcile;
use crate::context::ControllerEvents;
use crate::lldap;
use crate::resources::Error;

#[derive(Deserialize, Serialize, Clone, Copy, Debug, JsonSchema)]
pub enum Type {
    String,
    Integer,
    Jpeg,
    DateTime,
}

impl From<Type> for AttributeType {
    fn from(t: Type) -> Self {
        match t {
            Type::String => Self::String,
            Type::Integer => Self::Integer,
            Type::Jpeg => Self::JpegPhoto,
            Type::DateTime => Self::DateTime,
        }
    }
}

#[derive(CustomResource, Deserialize, Serialize, Clone, Debug, CELSchema)]
#[kube(
    kind = "UserAttribute",
    group = "lldap.huizinga.dev",
    version = "v1",
    status = "UserAttributesStatus"
)]
#[kube(
    shortname = "lua",
    doc = "Custom resource for managing custom User Attributes inside of LLDAP",
    printcolumn = r#"{"name":"Type", "type":"string", "description":"Type of attribute", "jsonPath":".spec.type"}"#,
    printcolumn = r#"{"name":"List", "type":"boolean", "description":"Can the attribute contain multiple values", "jsonPath":".spec.list"}"#,
    printcolumn = r#"{"name":"Visible", "type":"boolean", "description":"Can users see the value", "jsonPath":".spec.userVisible"}"#,
    printcolumn = r#"{"name":"Editable", "type":"boolean", "description":"Can users edit the value", "jsonPath":".spec.userEditable"}"#,
    printcolumn = r#"{"name":"Synced", "type":"boolean", "jsonPath":".status.synced"}"#,
    printcolumn = r#"{"name":"Age", "type":"date", "jsonPath":".metadata.creationTimestamp"}"#
)]
#[kube(
    rule = Rule::new("self.spec == oldSelf.spec").message("User attributes are immutable"),
    rule = Rule::new("!self.spec.userEditable || self.spec.userVisible && self.spec.userEditable").message("Editable attribute must also be visible")
)]
#[serde(rename_all = "camelCase")]
pub struct UesrAttributeSpec {
    r#type: Type,
    #[serde(default)]
    list: bool,
    #[serde(default)]
    user_visible: bool,
    #[serde(default)]
    user_editable: bool,
}
#[derive(Deserialize, Serialize, Clone, Default, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UserAttributesStatus {
    pub synced: bool,
}

impl Reconcile for UserAttribute {
    async fn reconcile(
        self: std::sync::Arc<Self>,
        ctx: std::sync::Arc<crate::context::Context>,
    ) -> super::Result<kube::runtime::controller::Action> {
        let name = self
            .metadata
            .name
            .clone()
            .ok_or(Error::MissingObjectKey(".metadata.name"))?;

        debug!(name, "Apply");

        trace!(name, "Get existing attributes");
        let lldap_client = ctx.lldap_config.build_client().await?;
        let user_attributes = lldap_client.get_user_attributes().await?;
        trace!("{user_attributes:?}");

        let client = &ctx.client;
        let api = Api::<UserAttribute>::all(client.clone());
        if let Some(attribute) = user_attributes
            .iter()
            .find(|attribute| attribute.name == name)
        {
            trace!("User attribute already exists: {attribute:?}");
            let mut desynced: Vec<String> = Vec::new();
            if attribute.attribute_type != self.spec.r#type.into() {
                desynced.push("type".into());
            }

            if attribute.is_list != self.spec.list {
                desynced.push("list".into());
            }

            if attribute.is_visible != self.spec.user_visible {
                desynced.push("userVisible".into());
            }

            if attribute.is_editable != self.spec.user_editable {
                desynced.push("userEditable".into());
            }

            if !desynced.is_empty() {
                set_status(&api, &name, false).await?;
                ctx.recorder
                    .user_attribute_desync(self.as_ref(), &desynced)
                    .await?;

                return Err(Error::UserAttributeDesync(desynced));
            }

            trace!("User attribute matches with spec");
        } else {
            trace!("User attribute does not exist yet");

            lldap_client
                .create_user_attribute(
                    &name,
                    self.spec.r#type,
                    self.spec.list,
                    self.spec.user_visible,
                    self.spec.user_editable,
                )
                .await?;
            ctx.recorder.user_attribute_created(self.as_ref()).await?;
        }

        set_status(&api, &name, true).await?;

        Ok(Action::requeue(Duration::from_secs(3600)))
    }

    async fn cleanup(
        self: std::sync::Arc<Self>,
        ctx: std::sync::Arc<crate::context::Context>,
    ) -> super::Result<kube::runtime::controller::Action> {
        let name = self
            .metadata
            .name
            .clone()
            .ok_or(Error::MissingObjectKey(".metadata.name"))?;

        debug!(name, "Cleanup");

        let lldap_client = ctx.lldap_config.build_client().await?;

        trace!(name, "Deleting user attribute");
        match lldap_client.delete_user_attribute(&name).await {
            Err(lldap::Error::GraphQl(err))
                if err.message == format!("Attribute {name} is not defined in the schema") =>
            {
                warn!(name, "User attribute not found");
                Ok(())
            }
            Ok(_) => {
                ctx.recorder.user_attribute_deleted(self.as_ref()).await?;
                Ok(())
            }
            Err(err) => Err(err),
        }?;

        Ok(Action::await_change())
    }
}

async fn set_status(
    api: &Api<UserAttribute>,
    name: &str,
    synced: bool,
) -> Result<UserAttribute, kube::Error> {
    trace!(name, "Updating status");
    let status = json!({
        "status": UserAttributesStatus { synced }
    });
    api.patch_status(name, &PatchParams::default(), &Patch::Merge(&status))
        .await
}
