extern crate prost_build;

use std::io::{stderr, Write};
use std::process;

fn main() {
    prost_build::compile_protos(
        &["proto/node.proto"],
        &["proto/"]
    ).unwrap_or_else(|e| {
        writeln!(stderr(), "{}", e).unwrap();
        process::exit(1);
    });
}
