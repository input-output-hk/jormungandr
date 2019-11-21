use chain_core::property::FromStr;
use chain_impl_mockchain::key::Hash;
use jormungandr_integration_tests::mock::server::{self, ProtocolVersion};
use std::{env, path::PathBuf, thread};

fn main() {
    let args: Vec<String> = env::args().collect();
    let port: u16 = args[1].parse().unwrap();

    let genesis_hash =
        Hash::from_str("6ebcedcaf48791a48ba1dcd59a2b33dbea3e22667f018a7ce9d66a89cfecec97").unwrap();
    let tip =
        Hash::from_str("1c3ad65daec5ccb157b439ecd5e8d0574e389077cc672dd2a256ab1af8e6a463").unwrap();
    let protocol = ProtocolVersion::GenesisPraos;

    let path_log_file = PathBuf::from("mock.log");

    let _server = server::start(
        port,
        genesis_hash,
        tip,
        protocol.clone(),
        path_log_file.clone(),
    );
    println!(
        "Mock server started with genesis hash: {}, tip hash: {}, and protocol: {}, log_file:{:?} ",
        genesis_hash,
        tip,
        protocol,
        path_log_file.as_os_str()
    );

    loop {
        thread::park();
    }
}
