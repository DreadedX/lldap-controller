use std::sync::Arc;
use std::time::Duration;

use futures::StreamExt;
use k8s_openapi::api::core::v1::Secret;
use kube::runtime::Controller;
use kube::runtime::controller::Action;
use kube::{Api, Client as KubeClient};
use lldap_controller::context::Context;
use lldap_controller::lldap::LldapConfig;
use lldap_controller::resources::{self, ServiceUser, reconcile};
use tracing::{debug, info, warn};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Registry};

fn error_policy(_obj: Arc<ServiceUser>, err: &resources::Error, _ctx: Arc<Context>) -> Action {
    warn!("error: {}", err);
    Action::requeue(Duration::from_secs(5))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let logger = tracing_subscriber::fmt::layer().json();
    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info"))
        .expect("Fallback should be valid");

    Registry::default().with(logger).with(env_filter).init();

    info!("Starting controller");

    let client = KubeClient::try_default().await?;

    let data = Context::new(
        "lldap.huizinga.dev",
        client.clone(),
        LldapConfig::try_from_env()?,
    );

    let service_users = Api::<ServiceUser>::all(client.clone());
    let secrets = Api::<Secret>::all(client.clone());

    Controller::new(service_users.clone(), Default::default())
        .owns(secrets, Default::default())
        .shutdown_on_signal()
        .run(reconcile, error_policy, Arc::new(data))
        .for_each(|res| async move {
            match res {
                Ok(obj) => debug!("reconciled {:?}", obj.0.name),
                Err(err) => warn!("reconcile failed: {}", err),
            }
        })
        .await;

    Ok(())
}
