#[cynic::schema("lldap")]
pub(crate) mod schema {}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(graphql_type = "Query")]
pub struct GetUserAttributes {
    pub schema: Schema,
}

#[derive(cynic::QueryFragment, Debug)]
pub struct Schema {
    pub user_schema: AttributeList,
}

#[derive(cynic::QueryFragment, Debug)]
pub struct AttributeList {
    pub attributes: Vec<AttributeSchema>,
}

#[derive(cynic::QueryFragment, Debug)]
pub struct AttributeSchema {
    pub name: String,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(graphql_type = "Mutation")]
pub struct CreateManagedUserAttribute {
    #[arguments(attributeType: "INTEGER", isEditable: false, isList: false, isVisible: false, name: "managed")]
    pub add_user_attribute: Success,
}

#[derive(cynic::QueryFragment, Debug)]
pub struct Success {
    pub ok: bool,
}

#[derive(cynic::Enum, Clone, Copy, Debug)]
pub enum AttributeType {
    String,
    Integer,
    JpegPhoto,
    DateTime,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(graphql_type = "Query")]
pub struct ListManagedUsers {
    #[arguments(filters: { eq: { field: "managed", value: "1" } })]
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

#[derive(cynic::QueryVariables, Debug)]
pub struct CreateUserVariables<'a> {
    pub id: &'a str,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(graphql_type = "Mutation", variables = "CreateUserVariables")]
pub struct CreateUser {
    #[arguments(user: { attributes: { name: "managed", value: "1" }, email: $id, id: $id })]
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

    #[test]
    fn get_user_attributes_gql_output() {
        use cynic::QueryBuilder;

        let operation = GetUserAttributes::build(());

        insta::assert_snapshot!(operation.query);
    }

    #[test]
    fn create_managed_user_attribute_gql_output() {
        use cynic::MutationBuilder;

        let operation = CreateManagedUserAttribute::build(());

        insta::assert_snapshot!(operation.query);
    }

    #[test]
    fn list_managed_users_gql_output() {
        use cynic::QueryBuilder;

        let operation = ListManagedUsers::build(());

        insta::assert_snapshot!(operation.query);
    }

    #[test]
    fn delete_user_gql_output() {
        use cynic::MutationBuilder;

        let operation = DeleteUser::build(DeleteUserVariables { id: "user" });

        insta::assert_snapshot!(operation.query);
    }

    #[test]
    fn create_user_gql_output() {
        use cynic::MutationBuilder;

        let operation = CreateUser::build(CreateUserVariables { id: "user" });

        insta::assert_snapshot!(operation.query);
    }

    #[test]
    fn add_user_to_group_gql_output() {
        use cynic::MutationBuilder;

        let operation = AddUserToGroup::build(AddUserToGroupVariables {
            id: "user",
            group: 3,
        });

        insta::assert_snapshot!(operation.query);
    }
}
