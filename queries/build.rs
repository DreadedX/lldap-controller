fn main() {
    cynic_codegen::register_schema("lldap")
        .from_sdl_file("schemas/lldap.graphql")
        .unwrap()
        .as_default()
        .unwrap();
}
