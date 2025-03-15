use std::{collections::BTreeMap, str::from_utf8, sync::Arc, time::Duration};

use cynic::{http::SurfExt, MutationBuilder, QueryBuilder};
use futures::StreamExt;
use k8s_openapi::api::core::v1::Secret;
use kube::{
    api::{ObjectMeta, Patch, PatchParams, PostParams},
    runtime::{
        controller::Action,
        events::{Event, EventType, Recorder, Reporter},
        Controller,
    },
    Api, Client as KubeClient, Resource,
};
use lldap_auth::login::{ClientSimpleLoginRequest, ServerLoginResponse};
use lldap_controller::{
    lldap::change_password,
    resources::{ServiceUser, ServiceUserStatus},
};
use passwords::PasswordGenerator;
use queries::{CreateUser, CreateUserVariables, ListUsers};
use serde_json::json;
use surf::{Client as SurfClient, Config, Url};
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

struct LldapConfig {
    username: String,
    password: String,
    url: String,
}

impl LldapConfig {
    async fn client(&self) -> std::result::Result<SurfClient, surf::Error> {
        let client: SurfClient = Config::new()
            .set_base_url(Url::parse(&self.url)?)
            .set_timeout(Some(Duration::from_secs(1)))
            .try_into()?;

        let response: ServerLoginResponse = client
            .post("/auth/simple/login")
            .body_json(&ClientSimpleLoginRequest {
                username: self.username.clone().into(),
                password: self.password.clone(),
            })?
            .recv_json()
            .await?;

        let client = client
            .config()
            .clone()
            .add_header("Authorization", format!("Bearer {}", response.token))?
            .try_into()?;

        Ok(client)
    }
}

struct Data {
    client: KubeClient,
    lldap: LldapConfig,
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
    let secret_name = format!("{name}-lldap-credentials");
    let namespace = obj
        .metadata
        .namespace
        .clone()
        .ok_or(Error::MissingObjectKey(".metadata.namespace"))?;
    let username = format!("{name}.{namespace}");
    let oref = obj.controller_owner_ref(&()).unwrap();

    debug!(name, "reconcile request");

    let client = &ctx.client;
    let secrets = Api::<Secret>::namespaced(client.clone(), &namespace);

    // TODO: Potentially issue: someone modifies the secret and removes the pass
    let mut created = false;
    let mut secret = secrets
        .entry(&secret_name)
        .await
        .map_err(Error::Kube)?
        .or_insert(|| {
            debug!(name, secret_name, "Generating new secret");

            let mut contents = BTreeMap::new();
            contents.insert("username".into(), username.clone());
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
                    note: Some(format!("Created secret '{secret_name}'")),
                    action: "SecretCreated".into(),
                    secondary: Some(secret.get().object_ref(&())),
                },
                &obj.object_ref(&()),
            )
            .await
            .map_err(Error::Kube)?;
    }

    let lldap_client = ctx.lldap.client().await.unwrap();

    let operation = ListUsers::build(());
    let response = lldap_client
        .post("/api/graphql")
        .run_graphql(operation)
        .await
        .unwrap();

    if !response
        .data
        .expect("Should get data")
        .users
        .iter()
        .any(|user| user.id == username)
    {
        let operation = CreateUser::build(CreateUserVariables { id: &username });
        lldap_client
            .post("/api/graphql")
            .run_graphql(operation)
            .await
            .unwrap();

        ctx.recorder
            .publish(
                &Event {
                    type_: EventType::Normal,
                    reason: "UserCreated".into(),
                    note: Some(format!("Created user '{username}'")),
                    action: "UserCreated".into(),
                    secondary: None,
                },
                &obj.object_ref(&()),
            )
            .await
            .map_err(Error::Kube)?;
    }

    let password = secret.get().data.as_ref().unwrap().get("password").unwrap();
    let password = from_utf8(&password.0).unwrap();
    change_password(&lldap_client, &username, password)
        .await
        .unwrap();

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

    let lldap = LldapConfig {
        username: std::env::var("LLDAP_USERNAME").unwrap(),
        password: std::env::var("LLDAP_PASSWORD").unwrap(),
        url: std::env::var("LLDAP_URL").unwrap(),
    };

    let client = KubeClient::try_default().await?;

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
                lldap,
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
