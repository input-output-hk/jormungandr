extern crate tower_grpc_build;

use std::io::{stderr, Write};
use std::process;

fn main() {
    tower_grpc_build::Config::new()
        .enable_client(true)
        .enable_server(true)
        .build(&["../network-proto/node.proto"], &["../network-proto/"])
        .unwrap_or_else(|e| {
            writeln!(stderr(), "{}", e).unwrap();
            process::exit(1)
        });
}
