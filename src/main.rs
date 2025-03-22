use std::sync::Arc;
use std::time::Duration;

use futures::StreamExt;
use k8s_openapi::api::core::v1::Secret;
use kube::runtime::controller::{self, Action};
use kube::runtime::reflector::ObjectRef;
use kube::runtime::{Controller, watcher};
use kube::{Api, Client as KubeClient, Resource};
use lldap_controller::context::Context;
use lldap_controller::lldap::LldapConfig;
use lldap_controller::resources::{self, Error, Group, ServiceUser, reconcile};
use tracing::{debug, info, warn};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Registry};

fn error_policy<T>(_obj: Arc<T>, err: &resources::Error, _ctx: Arc<Context>) -> Action {
    warn!("error: {}", err);
    Action::requeue(Duration::from_secs(5))
}

async fn log_status<T>(
    res: Result<(ObjectRef<T>, Action), controller::Error<Error, watcher::Error>>,
) where
    T: Resource,
{
    match res {
        Ok(obj) => debug!("reconciled {:?}", obj.0.name),
        Err(err) => warn!("reconcile failed: {}", err),
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info"))
        .expect("Fallback should be valid");

    if std::env::var("CARGO").is_ok() {
        let logger = tracing_subscriber::fmt::layer().compact();
        Registry::default().with(logger).with(env_filter).init();
    } else {
        let logger = tracing_subscriber::fmt::layer().json();
        Registry::default().with(logger).with(env_filter).init();
    }

    info!("Starting controller");

    let client = KubeClient::try_default().await?;

    let data = Context::new(
        "lldap.huizinga.dev",
        client.clone(),
        LldapConfig::try_from_env()?,
    );

    let service_users = Api::<ServiceUser>::all(client.clone());
    let secrets = Api::<Secret>::all(client.clone());

    let service_user_controller = Controller::new(service_users, Default::default())
        .owns(secrets, Default::default())
        .shutdown_on_signal()
        .run(reconcile, error_policy, Arc::new(data.clone()))
        .for_each(log_status);

    let groups = Api::<Group>::all(client.clone());

    let group_controller = Controller::new(groups, Default::default())
        .shutdown_on_signal()
        .run(reconcile, error_policy, Arc::new(data))
        .for_each(log_status);

    tokio::join!(service_user_controller, group_controller);

    Ok(())
}
