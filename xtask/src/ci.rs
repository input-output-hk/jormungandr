use xshell::cmd;

pub fn ci() {
    cmd!("cargo fmt -- --check").run().unwrap();
    cmd!("cargo clippy --all-features --all-targets -- -D warnings")
        .run()
        .unwrap();
    crate::test::test();
}
