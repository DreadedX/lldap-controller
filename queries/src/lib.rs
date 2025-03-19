#[cynic::schema("lldap")]
pub(crate) mod schema {}

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

#[derive(cynic::QueryVariables, Debug)]
pub struct GetUserVariables<'a> {
    pub id: &'a str,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(graphql_type = "Query", variables = "GetUserVariables")]
pub struct GetUser {
    #[arguments(userId: $id)]
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use cynic::MutationBuilder;
    use cynic::QueryBuilder;

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

    #[test]
    fn get_user_gql_output() {
        let operation = GetUser::build(GetUserVariables { id: "user" });

        insta::assert_snapshot!(operation.query);
    }
}
