use xshell::cmd;

pub fn test() {
    cmd!("cargo build -p jormungandr").run().unwrap();
    cmd!("cargo build -p jcli").run().unwrap();
    cmd!("cargo test").run().unwrap();
}
