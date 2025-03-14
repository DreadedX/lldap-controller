use std::{collections::BTreeMap, sync::Arc, time::Duration};

use futures::StreamExt;
use k8s_openapi::api::core::v1::Secret;
use kube::{
    api::{ObjectMeta, Patch, PatchParams, PostParams},
    runtime::{
        controller::Action,
        events::{Event, EventType, Recorder, Reporter},
        Controller,
    },
    Api, Client, Resource,
};
use lldap_controller::resources::{ServiceUser, ServiceUserStatus};
use passwords::PasswordGenerator;
use serde_json::json;
use tracing::{debug, info, instrument, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Registry};

#[derive(thiserror::Error, Debug)]
enum Error {
    #[error("Failed to commit secret: {0}")]
    Commit(#[source] kube::api::entry::CommitError),
    #[error("{0}")]
    Kube(#[source] kube::Error),
    #[error("MissingObjectKey: {0}")]
    MissingObjectKey(&'static str),
}

type Result<T, E = Error> = std::result::Result<T, E>;

struct Data {
    client: Client,
    recorder: Recorder,
    pg: PasswordGenerator,
}

const CONTROLLER_NAME: &str = "lldap.huizinga.dev";

#[instrument(skip(obj, ctx))]
async fn reconcile(obj: Arc<ServiceUser>, ctx: Arc<Data>) -> Result<Action> {
    let name = obj
        .metadata
        .name
        .clone()
        .ok_or(Error::MissingObjectKey(".metadata.name"))?;
    let namespace = obj
        .metadata
        .namespace
        .clone()
        .ok_or(Error::MissingObjectKey(".metadata.namespace"))?;
    let oref = obj.controller_owner_ref(&()).unwrap();

    debug!(name, "reconcile request");

    let client = &ctx.client;
    let secrets = Api::<Secret>::namespaced(client.clone(), &namespace);

    // TODO: Potentially issue: someone modifies the secret and removes the pass
    let mut created = false;
    let mut secret = secrets
        .entry(&name)
        .await
        .map_err(Error::Kube)?
        .or_insert(|| {
            debug!(name, "Generating new secret");

            let mut contents = BTreeMap::new();
            contents.insert("username".into(), name.clone());
            contents.insert("password".into(), ctx.pg.generate_one().unwrap());

            created = true;

            Secret {
                metadata: ObjectMeta {
                    owner_references: Some(vec![oref]),
                    ..Default::default()
                },
                string_data: Some(contents),
                ..Default::default()
            }
        });

    secret
        .commit(&PostParams {
            dry_run: false,
            field_manager: Some(CONTROLLER_NAME.into()),
        })
        .await
        .map_err(Error::Commit)?;
    let secret = secret;

    if created {
        debug!(name, "Sending SecretCreated event");

        // The reason this is here instead of inside the or_insert is that we
        // want to send the event _after_ it successfully committed.
        // Also or_insert is not async!
        ctx.recorder
            .publish(
                &Event {
                    type_: EventType::Normal,
                    reason: "SecretCreated".into(),
                    note: Some(format!("Created secret '{name}'")),
                    action: "NewSecret".into(),
                    secondary: Some(secret.get().object_ref(&())),
                },
                &obj.object_ref(&()),
            )
            .await
            .map_err(Error::Kube)?;
    }

    let service_users = Api::<ServiceUser>::namespaced(client.clone(), &namespace);
    let status = json!({
        "status": ServiceUserStatus { secret_created: secret.get().meta().creation_timestamp.as_ref().map(|ts| ts.0) }
    });
    service_users
        .patch_status(&name, &PatchParams::default(), &Patch::Merge(&status))
        .await
        .map_err(Error::Kube)?;

    Ok(Action::requeue(Duration::from_secs(3600)))
}

fn error_policy(_obj: Arc<ServiceUser>, err: &Error, _ctx: Arc<Data>) -> Action {
    warn!("error: {}", err);
    Action::requeue(Duration::from_secs(5))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let logger = tracing_subscriber::fmt::layer().json();
    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info"))
        .unwrap();

    Registry::default().with(logger).with(env_filter).init();

    info!("Starting controller");

    let client = Client::try_default().await?;

    let reporter: Reporter = CONTROLLER_NAME.into();
    let recorder = Recorder::new(client.clone(), reporter);

    let pg = PasswordGenerator::new()
        .length(32)
        .uppercase_letters(true)
        .strict(true);

    let service_users = Api::<ServiceUser>::all(client.clone());
    let secrets = Api::<Secret>::all(client.clone());

    Controller::new(service_users.clone(), Default::default())
        .owns(secrets, Default::default())
        .shutdown_on_signal()
        .run(
            reconcile,
            error_policy,
            Arc::new(Data {
                client,
                recorder,
                pg,
            }),
        )
        .for_each(|res| async move {
            match res {
                Ok(obj) => debug!("reconciled {:?}", obj.0.name),
                Err(err) => warn!("reconcile failed: {}", err),
            }
        })
        .await;

    Ok(())
}
