fn main() {
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
