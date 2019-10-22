extern crate base64;
extern crate chain_impl_mockchain;
extern crate futures;
extern crate futures_cpupool;
extern crate grpc;
extern crate hex;

use chain_core::mempack::ReadBuf;
use chain_core::mempack::Readable;
use chain_impl_mockchain as chain;
use jormungandr_integration_tests::mock::client::JormungandrClient;
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    let port: u16 = args[1].parse().unwrap();
    let client = JormungandrClient::new("127.0.0.1", port);

    let tip = client.get_tip();
    println!("tip hash: {:?}", tip);

    let hash = tip.hash();

    let blocks = client.get_blocks(&vec![hash]);
    blocks.iter().map(|i| println!("tip block: {:?}", i));
}
