fn main() {
    let production_prefix = option_env!("PRODUCTION_ADDRESS_PREFIX").unwrap_or("ca");
    let test_prefix = option_env!("TEST_ADDRESS_PREFIX").unwrap_or("ta");

    println!(
        "cargo:rustc-env=PRODUCTION_ADDRESS_PREFIX={}",
        production_prefix
    );
    println!("cargo:rustc-env=TEST_ADDRESS_PREFIX={}", test_prefix);
}
