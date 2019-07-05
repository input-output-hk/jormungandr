fn main() {
    let production_prefix = option_env!("ADDRESS_PREFIX").unwrap_or("ca");
    println!("cargo:rustc-env=ADDRESS_PREFIX={}", production_prefix);
}
