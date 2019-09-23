use crate::{
    client::JormungandrClient, node::HandshakeResponse, node::*, testing::read_into,
    testing::setup::Config,
};

use chain_core::property::FromStr;
use chain_impl_mockchain::{block::Block as ChainBlock, block::Header as ChainHeader, key::Hash};
// L1001 Handshake sanity
#[test]
pub fn handshake_sanity() {
    let client = Config::attach_to_local_node(9001).client();
    let handshake_response = client.handshake();

    println!("Recieved reponse:");
    println!("\tProtocol: {}", handshake_response.get_version());
    println!("\tBlock0: {}", hex::encode(handshake_response.get_block0()));
}

// L1006 Tip request
#[test]
pub fn tip_request() {
    let client = Config::attach_to_local_node(9001).client();
    let tip_response = client.get_tip();
    let header: ChainHeader = read_into(&tip_response.block_header);

    println!("Recieved header: {:?}", header);
}

// L1009 GetHeaders correct hash
#[test]
pub fn get_headers_correct_hash() {
    let client = Config::attach_to_local_node(9001).client();
    let headers_response = client.get_headers(vec![
        "46d57221f9c201558d4246fd7eff058e89464e7805bb5db73d7341f02a616d63",
    ]);

    print_stream_of_headers(headers_response);
}

// L1010 GetHeaders incorrect hash
#[test]
pub fn get_headers_incorrect_hash() {
    let client = Config::attach_to_local_node(9001).client();
    let headers_response = client.get_headers(vec![
        "efe2d4e5c4ad84b8e67e7b5676fff41cad5902a60b8cb6f072f42d7c7d26c944",
    ]);
    print_stream_of_headers(headers_response);
}

// L1011 GetBlocks correct hash
#[test]
pub fn get_blocks_correct_hash() {
    let client = Config::attach_to_local_node(9001).client();
    let blocks_response = client.get_blocks(vec![&Hash::from_str(
        "f3bb53f050fc175a508afe774fa1a8e7eb13c84949ab73915f2d9c52f319ed1f",
    )
    .unwrap()]);
    self::print_stream_of_blocks(blocks_response);
}
// L1012 GetBlocks incorrect hash
#[test]
pub fn get_blocks_incorrect_hash() {
    let client = Config::attach_to_local_node(9001).client();
    let blocks_response = client.get_blocks(vec![&Hash::from_str(
        "efe2d4e5c4ad84b8e67e7b5676fff41cad5902a60b8cb6f072f42d7c7d26c944",
    )
    .unwrap()]);
    self::print_stream_of_blocks(blocks_response);
}

// L1013 PullBlocksToTip correct hash
#[test]
pub fn pull_blocks_to_tip_correct_hash() {
    let client = Config::attach_to_local_node(9001).client();
    let blocks_response = client.pull_blocks_to_tip(
        Hash::from_str("6ebcedcaf48791a48ba1dcd59a2b33dbea3e22667f018a7ce9d66a89cfecec97").unwrap(),
    );

    self::print_stream_of_block_hashes(blocks_response);
}

// L1014 PullBlocksToTip incorrect hash
#[test]
pub fn pull_blocks_to_tip_incorrect_hash() {
    let client = Config::attach_to_local_node(9001).client();
    let blocks_response = client.pull_blocks_to_tip(
        Hash::from_str("bfe2d2e5c4ad84b8e67e7b5676fff41cad5902a60b8cb6f072f42d7c7d26c933").unwrap(),
    );

    self::print_stream_of_block_hashes(blocks_response);
}

// L1018 Pull headers correct hash
#[test]
pub fn pull_headers_correct_hash() {
    let client = Config::attach_to_local_node(9001).client();
    let headers_response = client.pull_headers(Some(
        Hash::from_str("6ebcedcaf48791a48ba1dcd59a2b33dbea3e22667f018a7ce9d66a89cfecec97").unwrap(),
    ));
    self::print_stream_of_headers(headers_response);
}

// L1019 Pull headers incorrect hash
#[test]
pub fn pull_headers_incorrect_hash() {
    let client = Config::attach_to_local_node(9001).client();
    let headers_response = client.pull_headers(Some(
        Hash::from_str("efe2d4e5c4ad84b8e67e7b5676fff41cad5902a60b8cb6f072f42d7c7d26c944").unwrap(),
    ));

    self::print_stream_of_headers(headers_response);
}

// L1019A Pull headers empty hash
#[test]
pub fn pull_headers_empty_hash() {
    let client = Config::attach_to_local_node(9001).client();
    let headers_response = client.pull_headers(None);
    self::print_stream_of_headers(headers_response);
}

// L1020 Push headers incorrect hash
#[test]
pub fn push_headers_incorrect_hash() {}

pub fn print_stream_of_headers(headers_response_stream: grpc::StreamingResponse<Header>) {
    println!("Recieved headers:");
    headers_response_stream
        .map_items(|i| {
            let header: ChainHeader = read_into(&i.content);
            println!("headers: {:?}", header);
        })
        .into_future()
        .wait()
        .unwrap();
}

pub fn print_stream_of_block_hashes(blocks_response_stream: grpc::StreamingResponse<Block>) {
    println!("Recieved headers:");
    blocks_response_stream
        .map_items(|i| {
            let block: ChainBlock = read_into(&i.content);
            println!("block_hash: {:?}", block.header.hash());
        })
        .into_future()
        .wait()
        .unwrap();
}

pub fn print_stream_of_blocks(block_response_stream: grpc::StreamingResponse<Block>) {
    println!("Recieved Blocks:");
    block_response_stream
        .map_items(|i| {
            let block: ChainBlock = read_into(&i.content);
            println!("block: {:?}", block);
        })
        .into_future()
        .wait()
        .unwrap();
}
