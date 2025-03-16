use anyhow::Context;
use lldap_auth::opaque::AuthenticationError;
use lldap_auth::registration::ServerRegistrationStartResponse;
use lldap_auth::{opaque, registration};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use std::time::Duration;
use tracing::debug;

use cynic::http::{CynicReqwestError, ReqwestExt};
use cynic::{GraphQlError, GraphQlResponse, MutationBuilder, QueryBuilder};
use lldap_auth::login::{ClientSimpleLoginRequest, ServerLoginResponse};
use queries::{CreateUser, CreateUserVariables, ListUsers};

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

fn check_graphql_errors<T>(response: &GraphQlResponse<T>) -> Result<()> {
    if let Some(errors) = &response.errors {
        if !errors.is_empty() {
            Err(errors.first().expect("Should not be empty").clone())?;
        }
    }

    Ok(())
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
    pub async fn list_users(&self) -> Result<impl Iterator<Item = String>> {
        let operation = ListUsers::build(());
        let response = self
            .client
            .post(format!("{}/api/graphql", self.url))
            .run_graphql(operation)
            .await?;

        check_graphql_errors(&response)?;

        Ok(response
            .data
            .expect("Data should be valid if there are no error")
            .users
            .into_iter()
            .map(|user| user.id))
    }

    pub async fn create_user(&self, username: &str) -> Result<()> {
        let operation = CreateUser::build(CreateUserVariables { id: username });

        // TODO: Check the response?
        let response = self
            .client
            .post(format!("{}/api/graphql", self.url))
            .run_graphql(operation)
            .await?;

        check_graphql_errors(&response)
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
