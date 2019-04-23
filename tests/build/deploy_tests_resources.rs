use std::{env, path::PathBuf};

pub const TARGET_DIRECTORY: &str = "./target/";

fn main() {
    let build_profile: PathBuf = env::var("PROFILE").unwrap().into();
    let output_directory = PathBuf::from(TARGET_DIRECTORY).join(build_profile);

    println!(
        "cargo:rustc-env=JCLI={}",
        output_directory.join("jcli").to_str().unwrap()
    );
    println!(
        "cargo:rustc-env=JORMUNGANDR={}",
        output_directory.join("jormungandr").to_str().unwrap()
    );
}
