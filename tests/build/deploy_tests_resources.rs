extern crate fs_extra;
use fs_extra::dir::{copy, CopyOptions};
use std::env;

pub const LOCAL_RESOURCE_DIRECTORY: &str = "./tests/resources";
pub const TARGET_DIRECTORY: &str = "./target/";

fn main() {
    let source_directory = LOCAL_RESOURCE_DIRECTORY;
    let build_profile = &env::var("PROFILE").unwrap();
    let output_directory = TARGET_DIRECTORY.to_owned() + build_profile;

    let mut options = CopyOptions::new();
    options.overwrite = true;

    //below line will make this build script to run each time
    //which prevent situation that output_directory was removed
    //and cargo build wouldn't create it
    //see: https://github.com/rust-lang/cargo/issues/4468#
    println!("cargo:rerun-if-changed=\"{}", &output_directory);

    println!(
        "Copying all resources from '{}' to '{}'",
        &source_directory, &output_directory
    );
    copy(&source_directory, &output_directory, &options).expect(&format!(
        "Cannot copy '{}' folder into {}",
        &source_directory, &output_directory
    ));
}
