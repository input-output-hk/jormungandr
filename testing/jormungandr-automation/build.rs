fn main() {
    tonic_build::compile_protos("proto/node.proto").unwrap();
    tonic_build::compile_protos("proto/watch.proto").unwrap();

    let jor_cli_name = option_env!("JOR_CLI_NAME").unwrap_or("jcli");
    let jormungandr_name = option_env!("JORMUNGANDR_NAME").unwrap_or("jormungandr");
    let jor_explorer_name = option_env!("JOR_EXPLORER_NAME").unwrap_or("explorer");
    println!("cargo:rustc-env=JOR_CLI_NAME={}", jor_cli_name);
    println!("cargo:rustc-env=JORMUNGANDR_NAME={}", jormungandr_name);
    println!("cargo:rustc-env=JOR_EXPLORER_NAME={}", jor_explorer_name);
    println!("cargo:rustc-env=RUST_BACKTRACE=full");

    let pkg_version = if let Ok(date) = std::env::var("DATE") {
        format!("{}.{}", env!("CARGO_PKG_VERSION"), date)
    } else {
        env!("CARGO_PKG_VERSION").to_string()
    };

    println!("cargo:rustc-env=CARGO_PKG_VERSION={}", pkg_version);

    let version = versionisator::Version::new(
        env!("CARGO_MANIFEST_DIR"),
        env!("CARGO_PKG_NAME").to_string(),
        pkg_version,
    );

    println!("cargo:rustc-env=FULL_VERSION={}", version.full());
    println!("cargo:rustc-env=SIMPLE_VERSION={}", version.simple());
    println!("cargo:rustc-env=SOURCE_VERSION={}", version.hash());
}
