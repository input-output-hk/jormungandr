fn main() {
    // generate grpc mock
    /*
    protoc_rust_grpc::Codegen::new()
        .out_dir("src/mock/proto")
        .includes(&["../../chain-deps/chain-network/proto"])
        .inputs(&["../../chain-deps/chain-network/proto/node.proto"])
        .rust_protobuf(true)
        .run()
        .expect("protoc-rust-grpc");
    */

    let jor_cli_name = option_env!("JOR_CLI_NAME").unwrap_or("jcli");
    let jormungandr_name = option_env!("JORMUNGANDR_NAME").unwrap_or("jormungandr");
    println!("cargo:rustc-env=JOR_CLI_NAME={}", jor_cli_name);
    println!("cargo:rustc-env=JORMUNGANDR_NAME={}", jormungandr_name);
    println!("cargo:rustc-env=RUST_BACKTRACE={}", "full");
}
