use crate::{node::HandshakeResponse, server, testing::setup::Config};

use std::thread;

use chain_core::property::FromStr;
use chain_impl_mockchain::key::Hash;
// L1005 Handshake version discrepancy
#[test]
pub fn wrong_protocol() {
    let _server = server::start(
        9002,
        Hash::from_str("6ebcedcaf48791a48ba1dcd59a2b33dbea3e22667f018a7ce9d66a89cfecec97").unwrap(),
        Hash::from_str("efe2d4e5c4ad84b8e67e7b5676fff41cad5902a60b8cb6f072f42d7c7d26c944").unwrap(),
        0,
    );

    loop {
        thread::park();
    }
}
// L1004 Handshake hash discrepancy
#[test]
pub fn wrong_genesis_hash() {
    let _server = server::start(
        9002,
        Hash::from_str("efe2d4e5c4ad84b8e67e7b5676fff41cad5902a60b8cb6f072f42d7c7d26c944").unwrap(),
        Hash::from_str("efe2d4e5c4ad84b8e67e7b5676fff41cad5902a60b8cb6f072f42d7c7d26c944").unwrap(),
        1,
    );

    loop {
        thread::park();
    }
}

// L1002 Handshake compatible
#[test]
pub fn handshake_ok() {
    let _server = server::start(
        9002,
        Hash::from_str("6ebcedcaf48791a48ba1dcd59a2b33dbea3e22667f018a7ce9d66a89cfecec97").unwrap(),
        Hash::from_str("efe2d4e5c4ad84b8e67e7b5676fff41cad5902a60b8cb6f072f42d7c7d26c944").unwrap(),
        1,
    );

    loop {
        thread::park();
    }
}
