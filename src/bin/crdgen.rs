use kube::CustomResourceExt;

fn main() {
    print!(
        "{}",
        serde_yaml::to_string(&lldap_controller::resources::ServiceUser::crd()).unwrap()
    )
}
