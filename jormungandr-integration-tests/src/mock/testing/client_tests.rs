
use crate::mock::{
    testing::{setup::bootstrap_node, setup::Config},
    read_into
};

use crate::common::{
    configuration::genesis_model::Fund, jcli_wrapper, jcli_wrapper::JCLITransactionWrapper,
    jormungandr::{logger::Level, Starter,ConfigurationBuilder},startup
};
use chain_core::property::FromStr;
use chain_impl_mockchain::{
    block::{Header,Block},
    key::Hash,
    testing::builders::{GenesisPraosBlockBuilder, StakePoolBuilder},
};
use chain_time::{Epoch, TimeEra};

// L1001 Handshake sanity
#[test]
pub fn handshake_sanity() {
    let (_server, config) = bootstrap_node();
    let client = Config::attach_to_local_node(config.node_config.get_p2p_port()).client();
    let handshake_response = client.handshake();

    assert_eq!(
        config.genesis_block_hash,
        hex::encode(handshake_response.get_block0()),
        "Genesis Block"
    );
    assert_eq!(handshake_response.get_version(), 1, "Protocol version");
}

// L1006 Tip request
#[test]
pub fn tip_request() {
    let (server, config) = bootstrap_node();
    let client = Config::attach_to_local_node(config.node_config.get_p2p_port()).client();

    let tip_header = client.get_tip();
    let block_hashes = server.logger.get_created_blocks_hashes();

    assert_eq!(*block_hashes.last().unwrap(), tip_header.hash());
}

// L1009 GetHeaders correct hash
#[test]
pub fn get_headers_correct_hash() {
    let (server, config) = bootstrap_node();
    let client = Config::attach_to_local_node(config.node_config.get_p2p_port()).client();

    let block_hashes = server.logger.get_created_blocks_hashes();
    let headers: Vec<Header> = response_to_vec!(client.get_headers(&block_hashes));
    let headers_hashes: Vec<Hash> = headers.iter()
                            .map(|x| x.hash())
                            .collect();
    assert_eq!(block_hashes, headers_hashes);
}

// L1010 GetHeaders incorrect hash
#[test]
pub fn get_headers_incorrect_hash() {
    let (_server, config) = bootstrap_node();
    let client = Config::attach_to_local_node(config.node_config.get_p2p_port()).client();
    let hash = Hash::from_str("efe2d4e5c4ad84b8e67e7b5676fff41cad5902a60b8cb6f072f42d7c7d26c944").unwrap();
    let headers_response: Vec<Header> = response_to_vec!(client.get_headers(&vec![hash]));
    assert!(headers_response.is_empty());
}

// L1011 GetBlocks correct hash
#[test]
pub fn get_blocks_correct_hash() {
    let (_server, config) = bootstrap_node();
    let client = Config::attach_to_local_node(config.node_config.get_p2p_port()).client();

    let tip = client.get_tip();
    let blocks: Vec<Block> = response_to_vec!(client.get_blocks(&vec![tip.hash()]));
    println!("{:?}", blocks);
}
// L1012 GetBlocks incorrect hash
#[test]
pub fn get_blocks_incorrect_hash() {
    let (_server, config) = bootstrap_node();
    let client = Config::attach_to_local_node(config.node_config.get_p2p_port()).client();

    let blocks: Vec<Block> = response_to_vec!(client.get_blocks(&vec![Hash::from_str(
        "efe2d4e5c4ad84b8e67e7b5676fff41cad5902a60b8cb6f072f42d7c7d26c944",
    )
    .unwrap()]));

    assert!(blocks.is_empty());
}

// L1013 PullBlocksToTip correct hash
#[test]
pub fn pull_blocks_to_tip_correct_hash() {
    let (server, config) = bootstrap_node();
    let client = Config::attach_to_local_node(config.node_config.get_p2p_port()).client();
    let blocks_headers: Vec<Block> = response_to_vec!(client
        .pull_blocks_to_tip(Hash::from_str(&config.genesis_block_hash).unwrap()));
    let blocks_hashes: Vec<Hash> = blocks_headers.iter()
        .map(|x| x.header.hash())
        .collect();

    let block_hashes_from_logs = server.logger.get_created_blocks_hashes();
    assert_eq!(block_hashes_from_logs, blocks_hashes);
}

// L1014 PullBlocksToTip incorrect hash
#[test]
pub fn pull_blocks_to_tip_incorrect_hash() {
    let (server, config) = bootstrap_node();
    let client = Config::attach_to_local_node(config.node_config.get_p2p_port()).client();
    let blocks: Vec<Block> = response_to_vec!(client.pull_blocks_to_tip(
        Hash::from_str("bfe2d2e5c4ad84b8e67e7b5676fff41cad5902a60b8cb6f072f42d7c7d26c933").unwrap(),
    ));

    let hashes_from_logs = server.logger.get_created_blocks_hashes();
    let block_hashes = blocks
        .iter()
        .map(|x| x.header.hash())
        .collect::<Vec<Hash>>();
    assert_eq!(
        hashes_from_logs, block_hashes,
        "If requested hash doesn't point to any block, all blocks should be returned"
    );
}

// L1018 Pull headers correct hash
#[test]
pub fn pull_headers_correct_hash() {
    let (server, config) = bootstrap_node();
    let client = Config::attach_to_local_node(config.node_config.get_p2p_port()).client();
    let tip_header = client.get_tip();
    let headers: Vec<Header> = response_to_vec!(client.pull_headers(None, Some(tip_header.hash())));
    let hashes: Vec<Hash> = headers.iter()
                                .map(|x| x.hash())
                                .collect();

    let hashes_from_logs = server.logger.get_created_blocks_hashes();
    assert_eq!(hashes, hashes_from_logs);
}

// L1019 Pull headers incorrect hash
#[test]
pub fn pull_headers_incorrect_hash() {
    let (_server, config) = bootstrap_node();
    let client = Config::attach_to_local_node(config.node_config.get_p2p_port()).client();
    let err = response_to_err!(client.pull_headers(
        Some(
            Hash::from_str("efe2d4e5c4ad84b8e67e7b5676fff41cad5902a60b8cb6f072f42d7c7d26c944")
                .unwrap(),
        ),
        None,
    ));
    match err {
        grpc::Error::GrpcMessage(grpc_error_message) => {
            assert_eq!(grpc_error_message.grpc_message, "invalid%20request%20data");
        }
        _ => panic!("Wrong error"),
    }
}

// L1019A Pull headers empty hash
#[test]
pub fn pull_headers_empty_hash() {
    let (_server, config) = bootstrap_node();
    let client = Config::attach_to_local_node(config.node_config.get_p2p_port()).client();
    let err = response_to_err!(client.pull_headers(None, None));
    match err {
        grpc::Error::GrpcMessage(grpc_error_message) => {
            assert_eq!(grpc_error_message.grpc_message, "invalid%20request%20data");
        }
        _ => panic!("Wrong error"),
    }
}

// L1020 Push headers incorrect header
#[test]
pub fn push_headers() {
    let (server, config) = bootstrap_node();
    let client = Config::attach_to_local_node(config.node_config.get_p2p_port()).client();
    let tip_header = client.get_tip();
    let stake_pool = StakePoolBuilder::new().build();

    let time_era = TimeEra::new(
        0u64.into(),
        Epoch(0u32),
        config
            .genesis_yaml
            .blockchain_configuration
            .slots_per_epoch
            .unwrap(),
    );

    let block = GenesisPraosBlockBuilder::new()
        .with_parent(&tip_header)
        .build(&stake_pool, &time_era);

    client.push_header(block.header);
    assert!(server
        .logger
        .get_lines_from_log()
        .any(|line| line.contains("not yet implemented")));
}

// L1020 Push headers incorrect header
#[test]
pub fn upload_block_incompatible_protocol() {
    let config = ConfigurationBuilder::new().with_slot_duration(4).build();
    let server = Starter::new().config(config.clone()).start().unwrap();
    let client = Config::attach_to_local_node(config.node_config.get_p2p_port()).client();
    let tip_header = client.get_tip();
    let stake_pool = StakePoolBuilder::new().build();

    let time_era = TimeEra::new(
        0u64.into(),
        Epoch(0u32),
        config
            .genesis_yaml
            .blockchain_configuration
            .slots_per_epoch
            .unwrap(),
    );

    let block = GenesisPraosBlockBuilder::new()
        .with_parent(&tip_header)
        .build(&stake_pool, &time_era);

    match client.upload_blocks(block).err().unwrap() {
        grpc::Error::GrpcMessage(grpc_error_message) => {
            assert_eq!(grpc_error_message.grpc_message, "invalid%20request%20data");
        }
        _ => panic!("Wrong error"),
    }

    assert!(server
        .logger
        .get_log_entries()
        .any(|entry| entry.level == Level::WARN
            && entry.task == Some("network".to_owned())
            && entry
                .msg
                .contains("block Version is incompatible with LeaderSelection")));
}

// L1020 Push headers incorrect header
#[test]
pub fn upload_block_nonexisting_stake_pool() {
    let config = ConfigurationBuilder::new()
        .with_slot_duration(4)
        .with_block0_consensus("genesis_praos")
        .build();
    let server = Starter::new().config(config.clone()).start().unwrap();
    let client = Config::attach_to_local_node(config.node_config.get_p2p_port()).client();
    let tip_header = client.get_tip();
    let stake_pool = StakePoolBuilder::new().build();

    let time_era = TimeEra::new(
        0u64.into(),
        Epoch(0u32),
        config
            .genesis_yaml
            .blockchain_configuration
            .slots_per_epoch
            .unwrap(),
    );

    let block = GenesisPraosBlockBuilder::new()
        .with_parent(&tip_header)
        .build(&stake_pool, &time_era);

    match client.upload_blocks(block).err().unwrap() {
        grpc::Error::GrpcMessage(grpc_error_message) => {
            assert_eq!(grpc_error_message.grpc_message, "invalid%20request%20data");
        }
        _ => panic!("Wrong error"),
    }

    assert!(server
        .logger
        .get_log_entries()
        .any(|entry| entry.level == Level::WARN
            && entry.task == Some("network".to_owned())
            && entry.msg.contains("Invalid block message")));
}

// L1020 Get fragments
#[test]
pub fn get_fragments() {
    let sender = startup::create_new_account_address();
    let receiver = startup::create_new_account_address();
    let output_value = 1u64;
    let config = ConfigurationBuilder::new()
        .with_slot_duration(4)
        .with_funds(vec![Fund {
            address: sender.address.clone(),
            value: 100.into(),
        }])
        .build();

    let server = Starter::new().config(config.clone()).start().unwrap();

    let transaction = JCLITransactionWrapper::new_transaction(&config.genesis_block_hash)
        .assert_add_account(&sender.address, &output_value.into())
        .assert_add_output(&receiver.address, &output_value.into())
        .assert_finalize()
        .seal_with_witness_for_address(&sender)
        .assert_to_message();

    let fragment_id =
        jcli_wrapper::assert_transaction_in_block(&transaction, &server.rest_address());
    let client = Config::attach_to_local_node(config.node_config.get_p2p_port()).client();
    match response_to_err!(client.get_fragments(vec![fragment_id.into_hash()])) {
        grpc::Error::Http(_) => (),
        _ => panic!("Wrong error"),
    };
    assert!(server
        .logger
        .get_lines_from_log()
        .any(|line| line.contains("not yet implemented")));

    /*assert_eq!(fragments.len(), 1);
    match fragments.iter().next().unwrap() {
        ChainFragment::Transaction(_tx) => (),
        _ => panic!("wrong fragment return from grpc"),
    }*/
}
