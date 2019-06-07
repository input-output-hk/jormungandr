use std::env;
use std::fs::rename;

fn main() {
    let jor_cli_name = option_env!("JOR_CLI_NAME").unwrap_or("jcli");
    println!("cargo:rustc-env=JOR_CLI_NAME={}", jor_cli_name);
}
