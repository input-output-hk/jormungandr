use jormungandr_integration_tests::mock::client::JormungandrClient;
use std::env;

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    let port: u16 = args[1].parse().unwrap();
    let client = JormungandrClient::new("127.0.0.1", port);
    let response = client.handshake().await;
    println!("{:?}", response);
}
