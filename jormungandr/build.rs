fn main() {
    let version = versionisator::Version::new(
        env!("CARGO_MANIFEST_DIR"),
        env!("CARGO_PKG_NAME").to_string(),
        env!("CARGO_PKG_VERSION").to_string(),
    );

    println!("cargo:rustc-env=FULL_VERSION={}", version.full());
    println!("cargo:rustc-env=SIMPLE_VERSION={}", version.simple());
    println!("cargo:rustc-env=SOURCE_VERSION={}", version.hash());
}
