mod group;
mod service_user;

use core::fmt;
use std::sync::Arc;

use kube::runtime::controller::Action;
use kube::runtime::finalizer;
use kube::{Api, Resource, ResourceExt};
use serde::Serialize;
use serde::de::DeserializeOwned;
use tracing::{debug, instrument};

pub use self::group::Group;
pub use self::service_user::ServiceUser;
use crate::context::Context;
use crate::lldap;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Failed to commit: {0}")]
    Commit(#[from] kube::api::entry::CommitError),
    #[error("Kube api error: {0}")]
    Kube(#[from] kube::Error),
    #[error("LLDAP error: {0}")]
    Lldap(#[from] lldap::Error),
    #[error("Finalizer error: {0}")]
    Finalizer(#[source] Box<finalizer::Error<Self>>),
    #[error("MissingObjectKey: {0}")]
    MissingObjectKey(&'static str),
}

impl From<finalizer::Error<Self>> for Error {
    fn from(error: finalizer::Error<Self>) -> Self {
        Self::Finalizer(Box::new(error))
    }
}

type Result<T, E = Error> = std::result::Result<T, E>;

trait Reconcile {
    async fn reconcile(self: Arc<Self>, ctx: Arc<Context>) -> Result<Action>;

    async fn cleanup(self: Arc<Self>, ctx: Arc<Context>) -> Result<Action>;
}

#[instrument(skip(obj, ctx))]
pub async fn reconcile<T>(obj: Arc<T>, ctx: Arc<Context>) -> Result<Action>
where
    T: Resource + ResourceExt + Clone + Serialize + DeserializeOwned + fmt::Debug + Reconcile,
    <T as Resource>::DynamicType: Default,
{
    debug!(name = obj.name_any(), "Reconcile");

    let service_users = Api::<T>::all(ctx.client.clone());

    Ok(
        finalizer(&service_users, &ctx.controller_name, obj, |event| async {
            match event {
                finalizer::Event::Apply(obj) => obj.reconcile(ctx.clone()).await,
                finalizer::Event::Cleanup(obj) => obj.cleanup(ctx.clone()).await,
            }
        })
        .await?,
    )
}
