use std::time::Duration;

use anyhow::anyhow;
use cynic::{http::SurfExt, MutationBuilder, QueryBuilder};
use lldap_controller::lldap::change_password;
use queries::{
    AddUserToGroup, AddUserToGroupVariables, CreateManagedUserAttribute, CreateUser,
    CreateUserVariables, DeleteUser, DeleteUserVariables, GetUserAttributes, ListManagedUsers,
};
use surf::{Client, Config, Url};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let token = std::env::var("LLDAP_TOKEN")?;

    let base_url = "http://localhost:17170";
    let users = [
        "authelia".to_owned(),
        "grafana".to_owned(),
        "gitea".to_owned(),
    ];

    let client: Client = Config::new()
        .set_base_url(Url::parse(base_url)?)
        .set_timeout(Some(Duration::from_secs(1)))
        .add_header("Authorization", format!("Bearer {token}"))
        .map_err(|e| anyhow!(e))?
        .try_into()?;

    let operation = GetUserAttributes::build(());
    let response = client
        .post("/api/graphql")
        .run_graphql(operation)
        .await
        .map_err(|e| anyhow!(e))?;

    let has_managed = response
        .data
        .as_ref()
        .expect("Should get data")
        .schema
        .user_schema
        .attributes
        .iter()
        .any(|attr| attr.name == "managed");

    if !has_managed {
        let operation = CreateManagedUserAttribute::build(());
        let _response = client
            .post("/api/graphql")
            .run_graphql(operation)
            .await
            .map_err(|e| anyhow!(e))?;
    }

    let operation = ListManagedUsers::build(());
    let response = client
        .post("/api/graphql")
        .run_graphql(operation)
        .await
        .map_err(|e| anyhow!(e))?;

    let (existing, remove): (Vec<_>, Vec<_>) = response
        .data
        .expect("Should get data")
        .users
        .into_iter()
        .map(|user| user.id)
        .partition(|user| users.contains(user));

    let (update, create): (Vec<_>, Vec<_>) = users.iter().partition(|user| existing.contains(user));

    for id in &remove {
        println!("Removing '{id}");

        let operation = DeleteUser::build(DeleteUserVariables { id });
        let _response = client
            .post("/api/graphql")
            .run_graphql(operation)
            .await
            .map_err(|e| anyhow!(e))?;
    }

    for id in create {
        println!("Creating '{id}'");

        let operation = CreateUser::build(CreateUserVariables { id });
        let _response = client
            .post("/api/graphql")
            .run_graphql(operation)
            .await
            .map_err(|e| anyhow!(e))?;

        let operation = AddUserToGroup::build(AddUserToGroupVariables { id, group: 3 });
        let _response = client
            .post("/api/graphql")
            .run_graphql(operation)
            .await
            .map_err(|e| anyhow!(e))?;

        change_password(&client, id, "JustATest").await?;
    }

    for id in update {
        println!("Updating '{id}'");

        change_password(&client, id, "JustATest").await?;
    }

    Ok(())
}
