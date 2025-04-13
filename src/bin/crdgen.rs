use kube::CustomResourceExt;

fn main() {
    let resources = [
        serde_yaml::to_string(&lldap_controller::resources::ServiceUser::crd()).unwrap(),
        serde_yaml::to_string(&lldap_controller::resources::Group::crd()).unwrap(),
        serde_yaml::to_string(&lldap_controller::resources::UserAttribute::crd()).unwrap(),
    ]
    .join("---\n");
    print!("{resources}")
}
