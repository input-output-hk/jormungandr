fn main() {
    let production_prefix = option_env!("PRODUCTION_ADDRESS_PREFIX").unwrap_or("ca");

    println!(
        "cargo:rustc-env=PRODUCTION_ADDRESS_PREFIX={}",
        production_prefix
    );
}
