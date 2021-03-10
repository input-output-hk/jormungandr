use chain_core::property::FromStr;
use chain_impl_mockchain::key::Hash;
use jormungandr_testing_utils::testing::node::grpc::server::{header, MockBuilder};
use std::env;

fn main() ->  {
    let tip_parent =
        Hash::from_str("1c3ad65daec5ccb157b439ecd5e8d0574e389077cc672dd2a256ab1af8e6a463").unwrap();

    let args: Vec<String> = env::args().collect();
    let port: u16 = args[1].parse().unwrap();

    let mut mock_controller = MockBuilder::new().with_port(port).build();

    std::thread::sleep(std::time::Duration::from_secs(60));
    mock_controller.set_tip(header(30, &tip_parent));
    std::thread::sleep(std::time::Duration::from_secs(60));
    mock_controller.stop();
}
