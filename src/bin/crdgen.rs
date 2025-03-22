use kube::CustomResourceExt;

fn main() {
    print!(
        "{}---\n{}",
        serde_yaml::to_string(&lldap_controller::resources::ServiceUser::crd()).unwrap(),
        serde_yaml::to_string(&lldap_controller::resources::Group::crd()).unwrap()
    )
}
