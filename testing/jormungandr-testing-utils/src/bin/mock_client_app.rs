use jormungandr_testing_utils::testing::node::grpc::client::JormungandrClient;
use rand::Rng;

use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    let port: u16 = args[2].parse().unwrap();
    let client = JormungandrClient::new(&args[1], port);
    let mut auth_nonce = [0u8; 32];
    rand::thread_rng().fill(&mut auth_nonce[..]);
    let response = client.handshake(&auth_nonce);
    println!("{:?}", response);
}
