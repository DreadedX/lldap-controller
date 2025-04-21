use k8s_openapi::api::core::v1::Secret;
use kube::runtime::events::{Event, EventType, Recorder, Reporter};
use kube::{Resource, ResourceExt};

use crate::lldap::LldapConfig;

#[derive(Clone)]
pub struct Context {
    pub client: kube::Client,
    pub lldap_config: LldapConfig,
    pub controller_name: String,
    pub recorder: Recorder,
    pub bind_dn_template: String,
}

impl Context {
    pub fn new(
        controller_name: &str,
        client: kube::Client,
        lldap_config: LldapConfig,
        bind_dn_template: impl Into<String>,
    ) -> Self {
        let reporter: Reporter = controller_name.into();
        let recorder = Recorder::new(client.clone(), reporter);

        Self {
            client,
            lldap_config,
            controller_name: controller_name.into(),
            recorder,
            bind_dn_template: bind_dn_template.into(),
        }
    }
}

#[allow(async_fn_in_trait)]
pub trait ControllerEvents {
    type Error;

    async fn secret_created<T>(&self, obj: &T, secret: &Secret) -> Result<(), Self::Error>
    where
        T: Resource<DynamicType = ()> + Sync;

    async fn user_created<T>(&self, obj: &T) -> Result<(), Self::Error>
    where
        T: Resource<DynamicType = ()> + Sync;

    async fn group_created<T>(&self, obj: &T) -> Result<(), Self::Error>
    where
        T: Resource<DynamicType = ()> + Sync;

    async fn user_deleted<T>(&self, obj: &T) -> Result<(), Self::Error>
    where
        T: Resource<DynamicType = ()> + Sync;

    async fn group_deleted<T>(&self, obj: &T) -> Result<(), Self::Error>
    where
        T: Resource<DynamicType = ()> + Sync;

    async fn user_attribute_created<T>(&self, obj: &T) -> Result<(), Self::Error>
    where
        T: Resource<DynamicType = ()> + Sync;

    async fn user_attribute_desync<T>(&self, obj: &T, fields: &[String]) -> Result<(), Self::Error>
    where
        T: Resource<DynamicType = ()> + Sync;

    async fn user_attribute_deleted<T>(&self, obj: &T) -> Result<(), Self::Error>
    where
        T: Resource<DynamicType = ()> + Sync;
}

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

    async fn user_created<T>(&self, obj: &T) -> Result<(), Self::Error>
    where
        T: Resource<DynamicType = ()> + Sync,
    {
        self.publish(
            &Event {
                type_: EventType::Normal,
                reason: "Created".into(),
                note: Some("Created user".into()),
                action: "Created".into(),
                secondary: None,
            },
            &obj.object_ref(&()),
        )
        .await
    }

    async fn group_created<T>(&self, obj: &T) -> Result<(), Self::Error>
    where
        T: Resource<DynamicType = ()> + Sync,
    {
        self.publish(
            &Event {
                type_: EventType::Normal,
                reason: "Created".into(),
                note: Some("Created group".into()),
                action: "Created".into(),
                secondary: None,
            },
            &obj.object_ref(&()),
        )
        .await
    }

    async fn user_deleted<T>(&self, obj: &T) -> Result<(), Self::Error>
    where
        T: Resource<DynamicType = ()> + Sync,
    {
        self.publish(
            &Event {
                type_: EventType::Normal,
                reason: "Deleted".into(),
                note: Some("Deleted user".into()),
                action: "Deleted".into(),
                secondary: None,
            },
            &obj.object_ref(&()),
        )
        .await
    }

    async fn group_deleted<T>(&self, obj: &T) -> Result<(), Self::Error>
    where
        T: Resource<DynamicType = ()> + Sync,
    {
        self.publish(
            &Event {
                type_: EventType::Normal,
                reason: "Deleted".into(),
                note: Some("Deleted group".into()),
                action: "Deleted".into(),
                secondary: None,
            },
            &obj.object_ref(&()),
        )
        .await
    }

    async fn user_attribute_created<T>(&self, obj: &T) -> Result<(), Self::Error>
    where
        T: Resource<DynamicType = ()> + Sync,
    {
        self.publish(
            &Event {
                type_: EventType::Warning,
                reason: "Created".into(),
                note: Some("Created user attribute".into()),
                action: "Created".into(),
                secondary: None,
            },
            &obj.object_ref(&()),
        )
        .await
    }

    async fn user_attribute_desync<T>(&self, obj: &T, fields: &[String]) -> Result<(), Self::Error>
    where
        T: Resource<DynamicType = ()> + Sync,
    {
        self.publish(
            &Event {
                type_: EventType::Warning,
                reason: "Desync".into(),
                note: Some(format!(
                    "User attribute fields '{fields:?}' are out of sync"
                )),
                action: "Desync".into(),
                secondary: None,
            },
            &obj.object_ref(&()),
        )
        .await
    }

    async fn user_attribute_deleted<T>(&self, obj: &T) -> Result<(), Self::Error>
    where
        T: Resource<DynamicType = ()> + Sync,
    {
        self.publish(
            &Event {
                type_: EventType::Warning,
                reason: "Deleted".into(),
                note: Some("Deleted user attribute'".into()),
                action: "Deleted".into(),
                secondary: None,
            },
            &obj.object_ref(&()),
        )
        .await
    }
}
