use anyhow::{anyhow, Context};
use lldap_auth::{opaque, registration};
use surf::Client;

pub async fn change_password(client: &Client, user_id: &str, password: &str) -> anyhow::Result<()> {
    let mut rng = rand::rngs::OsRng;
    let registration_start_request =
        opaque::client::registration::start_registration(password.as_bytes(), &mut rng)
            .context("Could not initiate password change")?;

    let start_request = registration::ClientRegistrationStartRequest {
        username: user_id.into(),
        registration_start_request: registration_start_request.message,
    };

    let mut response = client
        .post("/auth/opaque/register/start")
        .body_json(&start_request)
        .map_err(|e| anyhow!(e))?
        .await
        .map_err(|e| anyhow!(e))?;

    let response: registration::ServerRegistrationStartResponse =
        response.body_json().await.map_err(|e| anyhow!(e))?;

    let registration_finish = opaque::client::registration::finish_registration(
        registration_start_request.state,
        response.registration_response,
        &mut rng,
    )
    .context("Error during password change")?;

    let request = registration::ClientRegistrationFinishRequest {
        server_data: response.server_data,
        registration_upload: registration_finish.message,
    };

    let _response = client
        .post("/auth/opaque/register/finish")
        .body_json(&request)
        .map_err(|e| anyhow!(e))?
        .await
        .map_err(|e| anyhow!(e))?;

    println!("Changed '{user_id}' password successfully");

    Ok(())
}
