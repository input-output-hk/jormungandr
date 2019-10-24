extern crate protoc_rust;

use protoc_rust::Customize;
use std::env;

fn main() {
    // generate grpc mock

    /*protoc_rust::run(protoc_rust::Args {
        out_dir: "src/mock/proto",
        input: &["../../chain-libs/network-grpc/proto/node.proto"],
        includes: &["../../chain-libs/network-grpc/proto"],
        customize: Customize {
            ..Default::default()
        },
    })
    .expect("protoc");*/

    let jor_cli_name = option_env!("JOR_CLI_NAME").unwrap_or("jcli");
    let jormungandr_name = option_env!("JORMUNGANDR_NAME").unwrap_or("jormungandr");
    println!("cargo:rustc-env=JOR_CLI_NAME={}", jor_cli_name);
    println!("cargo:rustc-env=JORMUNGANDR_NAME={}", jormungandr_name);
    println!("cargo:rustc-env=RUST_BACKTRACE={}", "full");
}
