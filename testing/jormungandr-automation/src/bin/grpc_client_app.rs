use jormungandr_automation::jormungandr::grpc::client::JormungandrClient;
use rand::Rng;
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    let client = JormungandrClient::from_address(&format!("{}:{}", args[1], args[2])).unwrap();
    let mut auth_nonce = [0u8; 32];
    rand::thread_rng().fill(&mut auth_nonce[..]);
    let response = client.handshake(&auth_nonce);
    println!("{:?}", response);
}
