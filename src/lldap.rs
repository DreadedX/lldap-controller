use std::time::Duration;

use anyhow::Context;
use cynic::http::{CynicReqwestError, ReqwestExt};
use cynic::{GraphQlError, GraphQlResponse, MutationBuilder, QueryBuilder};
use lldap_auth::login::{ClientSimpleLoginRequest, ServerLoginResponse};
use lldap_auth::opaque::AuthenticationError;
use lldap_auth::registration::ServerRegistrationStartResponse;
use lldap_auth::{opaque, registration};
use queries::{
    AddUserToGroup, AddUserToGroupVariables, CreateUser, CreateUserVariables, DeleteUser,
    DeleteUserVariables, GetGroups, GetUser, GetUserVariables, Group, RemoveUserFromGroup,
    RemoveUserFromGroupVariables, User,
};
use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderValue};
use tracing::{debug, trace};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Cynic error: {0}")]
    Cynic(#[from] CynicReqwestError),
    #[error("Reqwest error: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("Authentication error: {0}")]
    Authentication(#[from] AuthenticationError),
    #[error("GraphQL error: {0}")]
    GraphQl(#[from] GraphQlError),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

fn check_graphql_errors<T>(response: GraphQlResponse<T>) -> Result<T> {
    if let Some(errors) = &response.errors {
        if !errors.is_empty() {
            Err(errors.first().expect("Should not be empty").clone())?;
        }
    }

    Ok(response
        .data
        .expect("Data should be valid if there are no error"))
}

pub struct LldapConfig {
    username: String,
    password: String,
    url: String,
}

impl LldapConfig {
    pub fn try_from_env() -> anyhow::Result<Self> {
        Ok(Self {
            username: std::env::var("LLDAP_USERNAME")
                .context("Variable 'LLDAP_USERNAME' is not set or invalid")?,
            password: std::env::var("LLDAP_PASSWORD")
                .context("Variable 'LLDAP_PASSWORD' is not set or invalid")?,
            url: std::env::var("LLDAP_URL")
                .context("Variable 'LLDAP_URL' is not set or invalid")?,
        })
    }

    pub async fn build_client(&self) -> Result<LldapClient> {
        debug!("Creating LLDAP client");
        let timeout = Duration::from_secs(1);

        let client = reqwest::ClientBuilder::new().timeout(timeout).build()?;

        let response: ServerLoginResponse = client
            .post(format!("{}/auth/simple/login", self.url))
            .json(&ClientSimpleLoginRequest {
                username: self.username.clone().into(),
                password: self.password.clone(),
            })
            .send()
            .await?
            .json()
            .await?;

        let mut auth: HeaderValue = format!("Bearer {}", response.token)
            .try_into()
            .expect("Token comes from api and should be ascii");
        auth.set_sensitive(true);
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, auth);

        let client = reqwest::ClientBuilder::new()
            .timeout(timeout)
            .default_headers(headers)
            .build()?;

        Ok(LldapClient {
            client,
            url: self.url.clone(),
        })
    }
}

pub struct LldapClient {
    client: reqwest::Client,
    url: String,
}

impl LldapClient {
    pub async fn get_user(&self, username: &str) -> Result<User> {
        let operation = GetUser::build(GetUserVariables { username });
        let response = self
            .client
            .post(format!("{}/api/graphql", self.url))
            .run_graphql(operation)
            .await?;

        Ok(check_graphql_errors(response)?.user)
    }

    pub async fn create_user(&self, username: &str) -> Result<User> {
        let operation = CreateUser::build(CreateUserVariables { username });

        let response = self
            .client
            .post(format!("{}/api/graphql", self.url))
            .run_graphql(operation)
            .await?;

        Ok(check_graphql_errors(response)?.create_user)
    }

    pub async fn delete_user(&self, username: &str) -> Result<()> {
        let operation = DeleteUser::build(DeleteUserVariables { username });

        let response = self
            .client
            .post(format!("{}/api/graphql", self.url))
            .run_graphql(operation)
            .await?;

        check_graphql_errors(response)?;

        Ok(())
    }

    pub async fn get_groups(&self) -> Result<Vec<Group>> {
        let operation = GetGroups::build(());

        let response = self
            .client
            .post(format!("{}/api/graphql", self.url))
            .run_graphql(operation)
            .await?;

        Ok(check_graphql_errors(response)?.groups)
    }

    pub async fn add_user_to_group(&self, username: &str, group: i32) -> Result<()> {
        let operation = AddUserToGroup::build(AddUserToGroupVariables { username, group });

        let response = self
            .client
            .post(format!("{}/api/graphql", self.url))
            .run_graphql(operation)
            .await?;

        check_graphql_errors(response)?;

        Ok(())
    }

    pub async fn remove_user_from_group(&self, username: &str, group: i32) -> Result<()> {
        let operation =
            RemoveUserFromGroup::build(RemoveUserFromGroupVariables { username, group });

        let response = self
            .client
            .post(format!("{}/api/graphql", self.url))
            .run_graphql(operation)
            .await?;

        check_graphql_errors(response)?;

        Ok(())
    }

    pub async fn update_user_groups(&self, user: &User, needed_groups: &[String]) -> Result<()> {
        let all_groups = self.get_groups().await?;

        // TODO: Error when invalid name
        let needed_groups: Vec<_> = needed_groups
            .iter()
            .filter_map(|needed_group| {
                all_groups
                    .iter()
                    .find(|group| &group.display_name == needed_group)
                    .map(|group| group.id)
            })
            .collect();

        let current_groups: Vec<_> = user.groups.iter().map(|group| group.id).collect();

        let remove = current_groups
            .iter()
            .filter(|group| !needed_groups.contains(group));
        for &group in remove {
            trace!(username = user.id, group, "Removing user from group");

            self.remove_user_from_group(&user.id, group).await?;
        }

        let add = needed_groups
            .iter()
            .filter(|group| !current_groups.contains(group));
        for &group in add {
            trace!(username = user.id, group, "Adding user to group");

            self.add_user_to_group(&user.id, group).await?;
        }

        Ok(())
    }

    pub async fn update_password(&self, username: &str, password: &str) -> Result<()> {
        let mut rng = rand::rngs::OsRng;
        let registration_start_request =
            opaque::client::registration::start_registration(password.as_bytes(), &mut rng)?;

        let start_request = registration::ClientRegistrationStartRequest {
            username: username.into(),
            registration_start_request: registration_start_request.message,
        };

        let response: ServerRegistrationStartResponse = self
            .client
            .post(format!("{}/auth/opaque/register/start", self.url))
            .json(&start_request)
            .send()
            .await?
            .json()
            .await?;

        let registration_finish = opaque::client::registration::finish_registration(
            registration_start_request.state,
            response.registration_response,
            &mut rng,
        )?;

        let request = registration::ClientRegistrationFinishRequest {
            server_data: response.server_data,
            registration_upload: registration_finish.message,
        };

        let _response = self
            .client
            .post(format!("{}/auth/opaque/register/finish", self.url))
            .json(&request)
            .send()
            .await?;

        debug!("Changed '{username}' password successfully");

        Ok(())
    }
}
