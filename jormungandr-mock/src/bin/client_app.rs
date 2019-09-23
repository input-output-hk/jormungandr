extern crate base64;
extern crate chain_impl_mockchain;
extern crate futures;
extern crate futures_cpupool;
extern crate grpc;
extern crate hex;

use crate::grpc::ClientStubExt;
use chain_core::mempack::ReadBuf;
use chain_core::mempack::Readable;
use chain_impl_mockchain as chain;
use jormungandr_mock::client::JormungandrClient;
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    let port: u16 = args[1].parse().unwrap();
    let client = JormungandrClient::new("127.0.0.1", port);

    let tip = client.get_tip();
    let mut buf = ReadBuf::from(tip.get_block_header());
    let header = chain::block::Header::read(&mut buf).unwrap();
    println!("header: {:?}", header);

    let hash = header.hash();

    let block_stream = client.get_blocks(vec![&hash]);
    block_stream
        .map_items(|i| {
            let mut buf = ReadBuf::from(&i.get_content());
            let block = chain::block::Block::read(&mut buf).unwrap();
            println!("block: {:?}", block);
        })
        .into_future()
        .wait()
        .unwrap();
}
