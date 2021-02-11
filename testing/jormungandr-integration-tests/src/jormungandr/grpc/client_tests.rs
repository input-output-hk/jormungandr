use crate::common::{
    jcli::JCli,
    jormungandr::{ConfigurationBuilder, Starter},
    startup,
    transaction_utils::TransactionHash,
};

use super::setup::{Config, Fixture};

use chain_core::property::FromStr;
use chain_crypto::{Ed25519, PublicKey, Signature, Verification};
use chain_impl_mockchain::{
    block::Header,
    chaintypes::ConsensusVersion,
    key::Hash,
    testing::{
        builders::{GenesisPraosBlockBuilder, StakePoolBuilder},
        TestGen,
    },
};
use chain_time::{Epoch, TimeEra};
use jormungandr_lib::interfaces::InitialUTxO;
use jormungandr_testing_utils::testing::node::grpc::client::MockClientError;
use tokio::time::sleep;

use assert_fs::TempDir;
use rand::Rng;
use std::time::Duration;

const CLIENT_RETRY_WAIT: Duration = Duration::from_millis(500);

// check that affix is a long enough (at least half the size) prefix of word
fn is_long_prefix<T: PartialEq>(word: &[T], affix: &[T]) -> bool {
    if word.len() < affix.len() || affix.len() * 2 < word.len() {
        return false;
    }
    affix.iter().zip(word.iter()).all(|(x, y)| x == y)
}

// L1001 Handshake sanity
#[tokio::test]
pub async fn handshake_sanity() {
    let fixture = Fixture::default();
    let (_server, config) = fixture.bootstrap_node();
    let client = Config::attach_to_local_node(config.get_p2p_listen_port()).client();
    let mut auth_nonce = [0u8; 32];
    rand::thread_rng().fill(&mut auth_nonce[..]);
    let handshake_response = client.handshake(&auth_nonce).await;

    assert_eq!(
        *config.genesis_block_hash(),
        hex::encode(handshake_response.block0),
        "Genesis Block"
    );
    assert_eq!(handshake_response.version, 1, "Protocol version");

    let public_key =
        PublicKey::<Ed25519>::from_binary(&handshake_response.node_id).expect("invalid node ID");
    let signature = Signature::<[u8], Ed25519>::from_binary(&handshake_response.signature)
        .expect("invalid signature");

    assert_eq!(
        signature.verify(&public_key, &auth_nonce),
        Verification::Success,
    );
}

// L1006 Tip request
#[tokio::test]
pub async fn tip_request() {
    let fixture = Fixture::default();
    let (server, config) = fixture.bootstrap_node();
    let client = Config::attach_to_local_node(config.get_p2p_listen_port()).client();

    let tip_header = client.tip().await;
    let block_hashes = server.logger.get_created_blocks_hashes();

    // TODO: this could fail if the server produces another block
    assert_eq!(*block_hashes.last().unwrap(), tip_header.hash());
}

// L1009 GetHeaders correct hash
#[tokio::test]
pub async fn get_headers_correct_hash() {
    let fixture = Fixture::new(1);
    let (server, config) = fixture.bootstrap_node();
    let client = Config::attach_to_local_node(config.get_p2p_listen_port()).client();

    sleep(Duration::from_secs(10)).await; // wait for the server to produce some blocks

    let block_hashes = server.logger.get_created_blocks_hashes();
    let headers: Vec<Header> = client
        .headers(&block_hashes)
        .await
        .expect("unexpected error returned");
    let headers_hashes: Vec<Hash> = headers.iter().map(|x| x.hash()).collect();
    assert!(is_long_prefix(&block_hashes, &headers_hashes));
}

// L1010 GetHeaders incorrect hash
#[tokio::test]
pub async fn get_headers_incorrect_hash() {
    let fixture = Fixture::default();
    let (_server, config) = fixture.bootstrap_node();
    let client = Config::attach_to_local_node(config.get_p2p_listen_port()).client();
    let fake_hash: Hash = TestGen::hash().into();
    assert_eq!(
        MockClientError::InvalidRequest(format!(
            "not found (block {} is not known to this node)",
            fake_hash.to_string()
        )),
        client.headers(&vec![fake_hash]).await.err().unwrap(),
        "wrong error"
    );
}

// L1011 GetBlocks correct hash
#[tokio::test]
pub async fn get_blocks_correct_hash() {
    let fixture = Fixture::default();
    let (_server, config) = fixture.bootstrap_node();
    let client = Config::attach_to_local_node(config.get_p2p_listen_port()).client();

    let tip = client.tip().await;
    assert!(client.get_blocks(&vec![tip.hash()]).await.is_ok());
}

// L1012 GetBlocks incorrect hash
#[tokio::test]
pub async fn get_blocks_incorrect_hash() {
    let fixture = Fixture::default();
    let (_server, config) = fixture.bootstrap_node();
    let client = Config::attach_to_local_node(config.get_p2p_listen_port()).client();
    let fake_hash: Hash = TestGen::hash().into();
    assert_eq!(
        MockClientError::InvalidRequest(format!(
            "not found (block {} is not known to this node)",
            fake_hash.to_string()
        )),
        client.headers(&vec![fake_hash]).await.err().unwrap(),
        "wrong error"
    );
}

// L1013 PullBlocksToTip correct hash
#[tokio::test]
pub async fn pull_blocks_to_tip_correct_hash() {
    let fixture = Fixture::new(1);
    let (server, config) = fixture.bootstrap_node();
    let client = Config::attach_to_local_node(config.get_p2p_listen_port()).client();

    sleep(Duration::from_secs(10)).await; // wait for the server to produce some blocks

    let blocks = client
        .pull_blocks_to_tip(Hash::from_str(config.genesis_block_hash()).unwrap())
        .await
        .unwrap();

    let blocks_hashes: Vec<Hash> = blocks.iter().map(|x| x.header.hash()).collect();

    let block_hashes_from_logs = server.logger.get_created_blocks_hashes();
    assert!(is_long_prefix(&block_hashes_from_logs, &blocks_hashes));
}

// L1014 PullBlocksToTip incorrect hash
#[tokio::test]
pub async fn pull_blocks_to_tip_incorrect_hash() {
    let fixture = Fixture::new(1);
    let (server, config) = fixture.bootstrap_node();
    let client = Config::attach_to_local_node(config.get_p2p_listen_port()).client();

    sleep(Duration::from_secs(10)).await; // wait for the server to produce some blocks

    let blocks = client
        .pull_blocks_to_tip(TestGen::hash().into())
        .await
        .unwrap();
    let blocks_hashes: Vec<Hash> = blocks.iter().map(|x| x.header.hash()).collect();

    let hashes_from_logs = server.logger.get_created_blocks_hashes();
    assert!(
        is_long_prefix(&hashes_from_logs, &blocks_hashes),
        "If requested hash doesn't point to any block, all blocks should be returned"
    );
}

// L1018 Pull headers correct hash
#[tokio::test]
pub async fn pull_headers_correct_hash() {
    let fixture = Fixture::new(1);
    let (server, config) = fixture.bootstrap_node();
    let client = Config::attach_to_local_node(config.get_p2p_listen_port()).client();

    sleep(Duration::from_secs(10)).await; // wait for the server to produce some blocks

    let tip_header = client.tip().await;
    let headers = client
        .pull_headers(&[client.get_genesis_block_hash().await], tip_header.hash())
        .await
        .unwrap();
    let hashes: Vec<Hash> = headers.iter().map(|x| x.hash()).collect();

    let hashes_from_logs = server.logger.get_created_blocks_hashes();
    assert!(is_long_prefix(&hashes, &hashes_from_logs));
}

// L1019 Pull headers incorrect hash
#[tokio::test]
pub async fn pull_headers_incorrect_hash() {
    let fixture = Fixture::default();
    let (_server, config) = fixture.bootstrap_node();
    let client = Config::attach_to_local_node(config.get_p2p_listen_port()).client();
    assert_eq!(
        MockClientError::InvalidRequest(format!("not found (block not found)")),
        client
            .pull_headers(&[], TestGen::hash().into())
            .await
            .err()
            .unwrap()
    );
}

// L1019A Pull headers empty hash
#[tokio::test]
pub async fn pull_headers_empty_start_hash() {
    let fixture = Fixture::new(1);
    let (server, config) = fixture.bootstrap_node();
    let client = Config::attach_to_local_node(config.get_p2p_listen_port()).client();

    sleep(Duration::from_secs(10)).await; // wait for the server to produce some blocks

    let tip_header = client.tip().await;
    let headers = client.pull_headers(&[], tip_header.hash()).await.unwrap();
    let hashes: Vec<Hash> = headers.iter().map(|x| x.hash()).collect();

    let hashes_from_logs = server.logger.get_created_blocks_hashes();
    assert!(is_long_prefix(&hashes_from_logs, &hashes));
}

// L1020 Push headers incorrect header
#[tokio::test]
pub async fn push_headers() {
    let fixture = Fixture::default();
    let (_server, config) = fixture.bootstrap_node();
    let client = Config::attach_to_local_node(config.get_p2p_listen_port()).client();
    let tip_header = client.tip().await;
    let stake_pool = StakePoolBuilder::new().build();

    let time_era = TimeEra::new(
        0u64.into(),
        Epoch(0u32),
        config
            .block0_configuration()
            .blockchain_configuration
            .slots_per_epoch
            .into(),
    );

    let block = GenesisPraosBlockBuilder::new()
        .with_parent(&tip_header)
        .build(&stake_pool, &time_era);

    assert!(client.push_headers(block.header).await.is_ok());
}

// L1020 Push headers incorrect header
#[tokio::test]
pub async fn upload_block_incompatible_protocol() {
    let temp_dir = TempDir::new().unwrap();

    let config = ConfigurationBuilder::new()
        .with_slot_duration(4)
        .build(&temp_dir);
    let _server = Starter::new().config(config.clone()).start().unwrap();
    let client = Config::attach_to_local_node(config.get_p2p_listen_port()).client();
    let tip_header = client.tip().await;
    let stake_pool = StakePoolBuilder::new().build();

    let time_era = TimeEra::new(
        0u64.into(),
        Epoch(0u32),
        config
            .block0_configuration()
            .blockchain_configuration
            .slots_per_epoch
            .into(),
    );

    let block = GenesisPraosBlockBuilder::new()
        .with_parent(&tip_header)
        .build(&stake_pool, &time_era);

    assert_eq!(
        MockClientError::InvalidRequest(
            format!("invalid request data (The block header verification failed: The block Version is incompatible with LeaderSelection.)") 
        ),
        client.upload_blocks(block.clone()).await.err().unwrap()
    );
}

// L1020 Push headers incorrect header
#[tokio::test]
pub async fn upload_block_nonexisting_stake_pool() {
    let temp_dir = TempDir::new().unwrap();

    let config = ConfigurationBuilder::new()
        .with_slot_duration(4)
        .with_block0_consensus(ConsensusVersion::GenesisPraos)
        .build(&temp_dir);
    let _server = Starter::new().config(config.clone()).start().unwrap();
    let client = Config::attach_to_local_node(config.get_p2p_listen_port()).client();
    let tip_header = client.tip().await;
    let stake_pool = StakePoolBuilder::new().build();

    let time_era = TimeEra::new(
        0u64.into(),
        Epoch(0u32),
        config
            .block0_configuration()
            .blockchain_configuration
            .slots_per_epoch
            .into(),
    );

    let block = GenesisPraosBlockBuilder::new()
        .with_parent(&tip_header)
        .build(&stake_pool, &time_era);

    assert_eq!(
        MockClientError::InvalidRequest(format!(
            "invalid request data (The block header verification failed: Invalid block message)"
        )),
        client.upload_blocks(block.clone()).await.err().unwrap()
    );
}

// L1020 Get fragments
#[tokio::test]
pub async fn get_fragments() {
    let temp_dir = TempDir::new().unwrap();
    let jcli: JCli = Default::default();

    let mut sender = startup::create_new_account_address();
    let receiver = startup::create_new_account_address();
    let output_value = 1u64;
    let config = ConfigurationBuilder::new()
        .with_slot_duration(4)
        .with_funds(vec![InitialUTxO {
            address: sender.address(),
            value: 100.into(),
        }])
        .build(&temp_dir);

    let server = Starter::new().config(config.clone()).start().unwrap();

    let transaction = sender
        .transaction_to(
            &server.genesis_block_hash(),
            &server.fees(),
            receiver.address(),
            output_value.into(),
        )
        .unwrap()
        .encode();

    let fragment_id = jcli
        .fragment_sender(&server)
        .send(&transaction)
        .assert_in_block();
    let client = Config::attach_to_local_node(config.get_p2p_listen_port()).client();
    println!("{:?}", client.get_fragments(vec![fragment_id.into()]).await);
}

// L1021 PullBlocks correct hashes
#[tokio::test]
pub async fn pull_blocks_correct_hashes_all_blocks() {
    let temp_dir = TempDir::new().unwrap();
    let config = ConfigurationBuilder::new()
        .with_slot_duration(1)
        .with_block0_consensus(ConsensusVersion::Bft)
        .build(&temp_dir);
    let server = Starter::new().config(config.clone()).start().unwrap();
    let client = Config::attach_to_local_node(config.get_p2p_listen_port()).client();

    sleep(Duration::from_secs(10)).await; // wait for the server to produce some blocks

    let genesis_block_hash = Hash::from_str(config.genesis_block_hash()).unwrap();
    let blocks = client
        .pull_blocks(&[genesis_block_hash], client.tip().await.id())
        .await
        .unwrap();

    let blocks_hashes: Vec<Hash> = blocks.iter().map(|x| x.header.hash()).collect();
    let block_hashes_from_logs = server.logger.get_created_blocks_hashes();
    assert!(is_long_prefix(&block_hashes_from_logs, &blocks_hashes));
}

// L1022 PullBlocks correct hashes
#[tokio::test]
pub async fn pull_blocks_correct_hashes_partial() {
    let temp_dir = TempDir::new().unwrap();
    let config = ConfigurationBuilder::new()
        .with_slot_duration(1)
        .with_block0_consensus(ConsensusVersion::Bft)
        .build(&temp_dir);
    let server = Starter::new().config(config.clone()).start().unwrap();

    let client = Config::attach_to_local_node(config.get_p2p_listen_port()).client();

    while client.tip().await.chain_length() < 10.into() {
        tokio::time::sleep(CLIENT_RETRY_WAIT).await;
    }

    let block_hashes_from_logs = server.logger.get_created_blocks_hashes();
    let start = 2;
    let end = 8;
    let expected_hashes = block_hashes_from_logs[start..end].to_vec();

    let blocks = client
        .pull_blocks(
            &[expected_hashes[0]],
            expected_hashes.last().copied().unwrap(),
        )
        .await
        .unwrap();

    let blocks_hashes: Vec<Hash> = blocks.iter().map(|x| x.header.hash()).collect();

    assert!(is_long_prefix(&expected_hashes[1..], &blocks_hashes));
}

// L1023 PullBlocks to and from in wrong order
#[tokio::test]
pub async fn pull_blocks_hashes_wrong_order() {
    let temp_dir = TempDir::new().unwrap();
    let config = ConfigurationBuilder::new()
        .with_slot_duration(1)
        .with_block0_consensus(ConsensusVersion::Bft)
        .build(&temp_dir);
    let server = Starter::new().config(config.clone()).start().unwrap();

    let client = Config::attach_to_local_node(config.get_p2p_listen_port()).client();

    while client.tip().await.chain_length() < 10.into() {
        tokio::time::sleep(CLIENT_RETRY_WAIT).await;
    }

    let block_hashes_from_logs = server.logger.get_created_blocks_hashes();
    let start = 2;
    let end = 8;
    let expected_hashes = block_hashes_from_logs[start..end].to_vec();

    let result = client
        .pull_blocks(
            &[expected_hashes.last().copied().unwrap()],
            expected_hashes[0],
        )
        .await;

    assert!(result.is_err());
}

// L1024 PullBlocks incorrect hashes
#[tokio::test]
pub async fn pull_blocks_incorrect_hashes() {
    let temp_dir = TempDir::new().unwrap();
    let config = ConfigurationBuilder::new()
        .with_slot_duration(1)
        .with_block0_consensus(ConsensusVersion::Bft)
        .build(&temp_dir);
    let _server = Starter::new().config(config.clone()).start().unwrap();

    let from = TestGen::hash();
    let to = TestGen::hash();

    let client = Config::attach_to_local_node(config.get_p2p_listen_port()).client();
    let result = client.pull_blocks(&[from], to).await;

    assert!(result.is_err());
}
