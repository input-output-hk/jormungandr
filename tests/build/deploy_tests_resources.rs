use std::{env, path::PathBuf};

pub const TARGET_DIRECTORY: &str = "./target/";

fn main() {
    let mut output_directory: PathBuf = env::var("OUT_DIR").unwrap().into();

    output_directory.pop();
    output_directory.pop();
    output_directory.pop();

    println!(
        "cargo:rustc-env=JCLI={}",
        output_directory.join("jcli").to_str().unwrap()
    );
    println!(
        "cargo:rustc-env=JORMUNGANDR={}",
        output_directory.join("jormungandr").to_str().unwrap()
    );
}
