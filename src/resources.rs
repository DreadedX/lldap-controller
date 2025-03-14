use chrono::{DateTime, Utc};
use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

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
    printcolumn = r#"{"name":"Age", "type":"date", "jsonPath":".metadata.creationTimestamp"}"#,
    printcolumn = r#"{"name":"Secret", "type":"date", "description":"Secret creation timestamp", "jsonPath":".status.secret_created"}"#
)]
#[serde(rename_all = "camelCase")]
pub struct ServiceUserSpec {
    #[serde(default)]
    password_manager: bool,
    #[serde(default)]
    additional_groups: Vec<String>,
}

#[derive(Deserialize, Serialize, Clone, Default, Debug, JsonSchema)]
pub struct ServiceUserStatus {
    pub secret_created: Option<DateTime<Utc>>,
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
