use chain_core::property::FromStr;
use chain_impl_mockchain::key::Hash;
use jormungandr_integration_tests::mock::server::{
    JormungandrServerImpl, NodeServer, ProtocolVersion,
};
use std::{env, path::PathBuf};
use tonic::transport::Server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let port: u16 = args[1].parse().unwrap();

    let genesis_hash =
        Hash::from_str("6ebcedcaf48791a48ba1dcd59a2b33dbea3e22667f018a7ce9d66a89cfecec97").unwrap();
    let tip =
        Hash::from_str("1c3ad65daec5ccb157b439ecd5e8d0574e389077cc672dd2a256ab1af8e6a463").unwrap();
    let protocol = ProtocolVersion::GenesisPraos;
    let path_log_file = PathBuf::from("mock.log");

    println!(
        "Mock server started with genesis hash: {}, tip hash: {}, and protocol: {}, log_file:{:?} ",
        genesis_hash,
        tip,
        protocol,
        path_log_file.as_os_str()
    );

    let addr = format!("127.0.0.1:{}", port);
    let mock = JormungandrServerImpl::new(port, genesis_hash, tip, protocol, path_log_file);

    Server::builder()
        .add_service(NodeServer::new(mock))
        .serve(addr.parse().unwrap())
        .await?;
    Ok(())
}
