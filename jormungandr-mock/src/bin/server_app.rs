extern crate base64;
extern crate futures;
extern crate futures_cpupool;
extern crate grpc;
extern crate hex;
extern crate protobuf;

use chain_core::property::FromStr;
use chain_impl_mockchain::key::Hash;
use futures::Stream;
use grpc::SingleResponse;
use jormungandr_mock::{node::*, node_grpc::*, server};
use std::{env, thread};

fn main() {
    let args: Vec<String> = env::args().collect();
    let port: u16 = args[1].parse().unwrap();

    let genesis_hash =
        Hash::from_str("6ebcedcaf48791a48ba1dcd59a2b33dbea3e22667f018a7ce9d66a89cfecec97").unwrap();
    let tip =
        Hash::from_str("1c3ad65daec5ccb157b439ecd5e8d0574e389077cc672dd2a256ab1af8e6a463").unwrap();
    let version = 1;

    let server = server::start(port, genesis_hash, tip, version);
    loop {
        thread::park();
    }
}
