extern crate fs_extra;
use fs_extra::dir::{copy, CopyOptions};
use std::{env, path::PathBuf};

pub const LOCAL_RESOURCE_DIRECTORY: &str = "./tests/resources";
pub const TARGET_DIRECTORY: &str = "./target/";

fn main() {
    let source_directory = LOCAL_RESOURCE_DIRECTORY;
    let build_profile: PathBuf = env::var("PROFILE").unwrap().into();
    let output_directory = PathBuf::from(TARGET_DIRECTORY).join(build_profile);

    let mut options = CopyOptions::new();
    options.overwrite = true;

    //below line will make this build script to run each time
    //which prevent situation that output_directory was removed
    //and cargo build wouldn't create it
    //see: https://github.com/rust-lang/cargo/issues/4468#
    println!(
        "cargo:rerun-if-changed={}",
        output_directory.to_str().unwrap()
    );
    println!(
        "cargo:rustc-env=JCLI={}",
        output_directory.join("jcli").to_str().unwrap()
    );
    println!(
        "cargo:rustc-env=JORMUNGANDR={}",
        output_directory.join("jormungandr").to_str().unwrap()
    );

    println!(
        "Copying all resources from '{:?}' to '{:?}'",
        &source_directory, &output_directory
    );
    copy(&source_directory, &output_directory, &options).expect(&format!(
        "Cannot copy '{:?}' folder into {:?}",
        &source_directory, &output_directory
    ));
}
