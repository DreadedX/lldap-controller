use std::sync::Arc;
use std::time::Duration;

use kube::CustomResource;
use kube::runtime::controller::Action;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tracing::{debug, trace};

use super::{Error, Reconcile, Result};
use crate::context::{Context, ControllerEvents};

#[derive(CustomResource, Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[kube(kind = "Group", group = "lldap.huizinga.dev", version = "v1")]
#[kube(
    shortname = "lg",
    doc = "Custom resource for managing Groups inside of LLDAP"
)]
#[serde(rename_all = "camelCase")]
pub struct GroupSpec {}

impl Reconcile for Group {
    async fn reconcile(self: Arc<Self>, ctx: Arc<Context>) -> Result<Action> {
        let name = self
            .metadata
            .name
            .clone()
            .ok_or(Error::MissingObjectKey(".metadata.name"))?;

        debug!(name, "Apply");

        let lldap_client = ctx.lldap_config.build_client().await?;

        trace!(name, "Get existing groups");
        let groups = lldap_client.get_groups().await?;

        if !groups.iter().any(|group| group.display_name == name) {
            trace!("Group does not exist yet");

            lldap_client.create_group(&name).await?;

            ctx.recorder.group_created(self.as_ref(), &name).await?;
        } else {
            trace!("Group already exists");
        }

        Ok(Action::requeue(Duration::from_secs(3600)))
    }

    async fn cleanup(self: Arc<Self>, ctx: Arc<Context>) -> Result<Action> {
        let name = self
            .metadata
            .name
            .clone()
            .ok_or(Error::MissingObjectKey(".metadata.name"))?;

        debug!(name, "Cleanup");

        let lldap_client = ctx.lldap_config.build_client().await?;

        trace!(name, "Get existing groups");
        let groups = lldap_client.get_groups().await?;

        if let Some(group) = groups.iter().find(|group| group.display_name == name) {
            trace!(name, "Deleting group");

            lldap_client.delete_group(group.id).await?;

            ctx.recorder.group_deleted(self.as_ref(), &name).await?;
        } else {
            trace!(name, "Group does not exist")
        }

        Ok(Action::await_change())
    }
}
