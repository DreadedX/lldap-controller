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

#[derive(cynic::QueryVariables, Debug)]
pub struct CreateGroupVariables<'a> {
    pub name: &'a str,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(graphql_type = "Mutation", variables = "CreateGroupVariables")]
pub struct CreateGroup {
    #[arguments(name: $name)]
    pub create_group: Group,
}

#[derive(cynic::QueryVariables, Debug)]
pub struct DeleteGroupVariables {
    pub id: i32,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(graphql_type = "Mutation", variables = "DeleteGroupVariables")]
pub struct DeleteGroup {
    #[arguments(groupId: $id)]
    pub delete_group: Success,
}

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
    pub is_visible: bool,
    pub is_list: bool,
    pub is_editable: bool,
    pub attribute_type: AttributeType,
}

#[derive(cynic::Enum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttributeType {
    String,
    Integer,
    JpegPhoto,
    DateTime,
}

#[derive(cynic::QueryVariables, Debug)]
pub struct CreateUserAttributeVariables<'a> {
    pub editable: bool,
    pub list: bool,
    pub name: &'a str,
    #[cynic(rename = "type")]
    pub r#type: AttributeType,
    pub visible: bool,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(graphql_type = "Mutation", variables = "CreateUserAttributeVariables")]
pub struct CreateUserAttribute {
    #[arguments(attributeType: $r#type, isEditable: $editable, isList: $list, isVisible: $visible, name: $name)]
    pub add_user_attribute: Success,
}

#[derive(cynic::QueryVariables, Debug)]
pub struct DeleteUserAttributeVariables<'a> {
    pub name: &'a str,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(graphql_type = "Mutation", variables = "DeleteUserAttributeVariables")]
pub struct DeleteUserAttribute {
    #[arguments(name: $name)]
    pub delete_user_attribute: Success,
}

#[cfg(test)]
mod tests {
    use cynic::{MutationBuilder, QueryBuilder};

    use super::*;

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

    #[test]
    fn create_group_gql_output() {
        let operation = CreateGroup::build(CreateGroupVariables { name: "group" });

        insta::assert_snapshot!(operation.query);
    }

    #[test]
    fn delete_group_gql_output() {
        let operation = DeleteGroup::build(DeleteGroupVariables { id: 0 });

        insta::assert_snapshot!(operation.query);
    }

    #[test]
    fn get_user_attributes_gql_output() {
        let operation = GetUserAttributes::build(());

        insta::assert_snapshot!(operation.query);
    }

    #[test]
    fn create_user_attribute_gql_output() {
        let operation = CreateUserAttribute::build(CreateUserAttributeVariables {
            r#type: AttributeType::String,
            list: true,
            editable: true,
            visible: true,
            name: "attr",
        });

        insta::assert_snapshot!(operation.query);
    }

    #[test]
    fn delete_user_attribute_gql_output() {
        let operation = DeleteUserAttribute::build(DeleteUserAttributeVariables { name: "attr" });

        insta::assert_snapshot!(operation.query);
    }
}
