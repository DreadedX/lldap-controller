use async_trait::async_trait;
use k8s_openapi::api::core::v1::Secret;
use kube::{
    runtime::events::{Event, EventType, Recorder, Reporter},
    Resource, ResourceExt,
};

use crate::lldap::LldapConfig;

pub struct Context {
    pub client: kube::Client,
    pub lldap_config: LldapConfig,
    pub controller_name: String,
    pub recorder: Recorder,
}

impl Context {
    pub fn new(controller_name: &str, client: kube::Client, lldap_config: LldapConfig) -> Self {
        let reporter: Reporter = controller_name.into();
        let recorder = Recorder::new(client.clone(), reporter);

        Self {
            client,
            lldap_config,
            controller_name: controller_name.into(),
            recorder,
        }
    }
}

#[async_trait]
pub trait ControllerEvents {
    type Error;

    async fn secret_created<T>(&self, obj: &T, secret: &Secret) -> Result<(), Self::Error>
    where
        T: Resource<DynamicType = ()> + Sync;

    async fn user_created<T>(&self, obj: &T, username: &str) -> Result<(), Self::Error>
    where
        T: Resource<DynamicType = ()> + Sync;

    async fn user_deleted<T>(&self, obj: &T, username: &str) -> Result<(), Self::Error>
    where
        T: Resource<DynamicType = ()> + Sync;
}

#[async_trait]
impl ControllerEvents for Recorder {
    type Error = kube::Error;

    async fn secret_created<T>(&self, obj: &T, secret: &Secret) -> Result<(), Self::Error>
    where
        T: Resource<DynamicType = ()> + Sync,
    {
        self.publish(
            &Event {
                type_: EventType::Normal,
                reason: "SecretCreated".into(),
                note: Some(format!("Created secret '{}'", secret.name_any())),
                action: "SecretCreated".into(),
                secondary: Some(secret.object_ref(&())),
            },
            &obj.object_ref(&()),
        )
        .await
    }

    async fn user_created<T>(&self, obj: &T, username: &str) -> Result<(), Self::Error>
    where
        T: Resource<DynamicType = ()> + Sync,
    {
        self.publish(
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
    }

    async fn user_deleted<T>(&self, obj: &T, username: &str) -> Result<(), Self::Error>
    where
        T: Resource<DynamicType = ()> + Sync,
    {
        self.publish(
            &Event {
                type_: EventType::Normal,
                reason: "UserDeleted".into(),
                note: Some(format!("Deleted user '{username}'")),
                action: "UserDeleted".into(),
                secondary: None,
            },
            &obj.object_ref(&()),
        )
        .await
    }
}
