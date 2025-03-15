#[cynic::schema("lldap")]
pub(crate) mod schema {}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(graphql_type = "Query")]
pub struct ListUsers {
    pub users: Vec<User>,
}

#[derive(cynic::QueryFragment, Debug)]
pub struct User {
    pub id: String,
}

#[derive(cynic::QueryVariables, Debug)]
pub struct DeleteUserVariables<'a> {
    pub id: &'a str,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(graphql_type = "Mutation", variables = "DeleteUserVariables")]
pub struct DeleteUser {
    #[arguments(userId: $id)]
    pub delete_user: Success,
}

#[derive(cynic::QueryFragment, Debug)]
pub struct Success {
    pub ok: bool,
}

#[derive(cynic::QueryVariables, Debug)]
pub struct CreateUserVariables<'a> {
    pub id: &'a str,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(graphql_type = "Mutation", variables = "CreateUserVariables")]
pub struct CreateUser {
    #[arguments(user: { email: $id, id: $id })]
    pub create_user: User,
}

#[derive(cynic::QueryVariables, Debug)]
pub struct AddUserToGroupVariables<'a> {
    pub group: i32,
    pub id: &'a str,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(graphql_type = "Mutation", variables = "AddUserToGroupVariables")]
pub struct AddUserToGroup {
    #[arguments(groupId: $group, userId: $id)]
    pub add_user_to_group: Success,
}

#[cfg(test)]
mod tests {
    use super::*;
    use cynic::MutationBuilder;
    use cynic::QueryBuilder;

    #[test]
    fn list_users_gql_output() {
        let operation = ListUsers::build(());

        insta::assert_snapshot!(operation.query);
    }

    #[test]
    fn delete_user_gql_output() {
        let operation = DeleteUser::build(DeleteUserVariables { id: "user" });

        insta::assert_snapshot!(operation.query);
    }

    #[test]
    fn create_user_gql_output() {
        let operation = CreateUser::build(CreateUserVariables { id: "user" });

        insta::assert_snapshot!(operation.query);
    }

    #[test]
    fn add_user_to_group_gql_output() {
        let operation = AddUserToGroup::build(AddUserToGroupVariables {
            id: "user",
            group: 3,
        });

        insta::assert_snapshot!(operation.query);
    }
}
