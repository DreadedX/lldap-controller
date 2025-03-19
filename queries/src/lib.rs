#[cynic::schema("lldap")]
pub(crate) mod schema {}

#[derive(cynic::QueryVariables, Debug)]
pub struct DeleteUserVariables<'a> {
    pub username: &'a str,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(graphql_type = "Mutation", variables = "DeleteUserVariables")]
pub struct DeleteUser {
    #[arguments(userId: $username)]
    pub delete_user: Success,
}

#[derive(cynic::QueryFragment, Debug)]
pub struct Success {
    pub ok: bool,
}

#[derive(cynic::QueryVariables, Debug)]
pub struct CreateUserVariables<'a> {
    pub username: &'a str,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(graphql_type = "Mutation", variables = "CreateUserVariables")]
pub struct CreateUser {
    #[arguments(user: { email: $username, id: $username })]
    pub create_user: User,
}

#[derive(cynic::QueryVariables, Debug)]
pub struct AddUserToGroupVariables<'a> {
    pub group: i32,
    pub username: &'a str,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(graphql_type = "Mutation", variables = "AddUserToGroupVariables")]
pub struct AddUserToGroup {
    #[arguments(groupId: $group, userId: $username)]
    pub add_user_to_group: Success,
}

#[derive(cynic::QueryVariables, Debug)]
pub struct RemoveUserFromGroupVariables<'a> {
    pub group: i32,
    pub username: &'a str,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(graphql_type = "Mutation", variables = "RemoveUserFromGroupVariables")]
pub struct RemoveUserFromGroup {
    #[arguments(groupId: $group, userId: $username)]
    pub remove_user_from_group: Success,
}

#[derive(cynic::QueryVariables, Debug)]
pub struct GetUserVariables<'a> {
    pub username: &'a str,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(graphql_type = "Query", variables = "GetUserVariables")]
pub struct GetUser {
    #[arguments(userId: $username)]
    pub user: User,
}

#[derive(cynic::QueryFragment, Debug)]
pub struct User {
    pub id: String,
    pub groups: Vec<Group>,
}

#[derive(cynic::QueryFragment, Debug)]
pub struct Group {
    pub id: i32,
    pub display_name: String,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(graphql_type = "Query")]
pub struct GetGroups {
    pub groups: Vec<Group>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use cynic::MutationBuilder;
    use cynic::QueryBuilder;

    #[test]
    fn delete_user_gql_output() {
        let operation = DeleteUser::build(DeleteUserVariables { username: "user" });

        insta::assert_snapshot!(operation.query);
    }

    #[test]
    fn create_user_gql_output() {
        let operation = CreateUser::build(CreateUserVariables { username: "user" });

        insta::assert_snapshot!(operation.query);
    }

    #[test]
    fn add_user_to_group_gql_output() {
        let operation = AddUserToGroup::build(AddUserToGroupVariables {
            username: "user",
            group: 3,
        });

        insta::assert_snapshot!(operation.query);
    }

    #[test]
    fn remove_user_from_group_gql_output() {
        let operation = RemoveUserFromGroup::build(RemoveUserFromGroupVariables {
            group: 3,
            username: "user",
        });

        insta::assert_snapshot!(operation.query);
    }

    #[test]
    fn get_user_gql_output() {
        let operation = GetUser::build(GetUserVariables { username: "user" });

        insta::assert_snapshot!(operation.query);
    }

    #[test]
    fn get_groups_gql_output() {
        let operation = GetGroups::build(());

        insta::assert_snapshot!(operation.query);
    }
}
