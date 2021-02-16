use super::setup;
use crate::common::{
    jcli::JCli, jormungandr::ConfigurationBuilder, startup, transaction_utils::TransactionHash,
};
use chain_core::property::FromStr;
use chain_crypto::{Ed25519, PublicKey, Signature, Verification};
use chain_impl_mockchain::chaintypes::ConsensusVersion;
use chain_impl_mockchain::{
    block::Header,
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
    let setup = setup::client::default().await;
    let mut auth_nonce = [0u8; 32];
    rand::thread_rng().fill(&mut auth_nonce[..]);
    let handshake_response = setup.client.handshake(&auth_nonce).await;

    assert_eq!(
        *setup.config.genesis_block_hash(),
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
    let setup = setup::client::default().await;

    let tip_header = setup.client.tip().await;
    let block_hashes = setup.server.logger.get_created_blocks_hashes();

    // TODO: this could fail if the server produces another block
    assert_eq!(*block_hashes.last().unwrap(), tip_header.hash());
}

// L1009 GetHeaders correct hash
#[tokio::test]
pub async fn get_headers_correct_hash() {
    let setup = setup::client::default().await;

    sleep(Duration::from_secs(10)).await; // wait for the server to produce some blocks

    let block_hashes = setup.server.logger.get_created_blocks_hashes();
    let headers: Vec<Header> = setup
        .client
        .headers(&block_hashes)
        .await
        .expect("unexpected error returned");
    let headers_hashes: Vec<Hash> = headers.iter().map(|x| x.hash()).collect();
    assert!(is_long_prefix(&block_hashes, &headers_hashes));
}

// L1010 GetHeaders incorrect hash
#[tokio::test]
pub async fn get_headers_incorrect_hash() {
    let setup = setup::client::default().await;
    let fake_hash: Hash = TestGen::hash();
    assert_eq!(
        MockClientError::InvalidRequest(format!(
            "not found (block {} is not known to this node)",
            fake_hash.to_string()
        )),
        setup.client.headers(&[fake_hash]).await.err().unwrap(),
        "wrong error"
    );
}

// L1011 GetBlocks correct hash
#[tokio::test]
pub async fn get_blocks_correct_hash() {
    let setup = setup::client::default().await;

    let tip = setup.client.tip().await;
    assert!(setup.client.get_blocks(&[tip.hash()]).await.is_ok());
}

// L1012 GetBlocks incorrect hash
#[tokio::test]
pub async fn get_blocks_incorrect_hash() {
    let setup = setup::client::default().await;
    let fake_hash: Hash = TestGen::hash();
    assert_eq!(
        MockClientError::InvalidRequest(format!(
            "not found (block {} is not known to this node)",
            fake_hash.to_string()
        )),
        setup.client.headers(&[fake_hash]).await.err().unwrap(),
        "wrong error"
    );
}

// L1013 PullBlocksToTip correct hash
#[tokio::test]
pub async fn pull_blocks_to_tip_correct_hash() {
    let setup = setup::client::default().await;

    sleep(Duration::from_secs(10)).await; // wait for the server to produce some blocks

    let blocks = setup
        .client
        .pull_blocks_to_tip(Hash::from_str(setup.config.genesis_block_hash()).unwrap())
        .await
        .unwrap();

    let blocks_hashes: Vec<Hash> = blocks.iter().map(|x| x.header.hash()).collect();

    let block_hashes_from_logs = setup.server.logger.get_created_blocks_hashes();
    assert!(is_long_prefix(&block_hashes_from_logs, &blocks_hashes));
}

// L1014 PullBlocksToTip incorrect hash
#[tokio::test]
#[ignore]
pub async fn pull_blocks_to_tip_incorrect_hash() {
    let setup = setup::client::default().await;

    sleep(Duration::from_secs(10)).await; // wait for the server to produce some blocks

    let blocks = setup
        .client
        .pull_blocks_to_tip(TestGen::hash())
        .await
        .unwrap();
    let blocks_hashes: Vec<Hash> = blocks.iter().map(|x| x.header.hash()).collect();

    let hashes_from_logs = setup.server.logger.get_created_blocks_hashes();
    assert!(
        is_long_prefix(&hashes_from_logs, &blocks_hashes),
        "If requested hash doesn't point to any block, all blocks should be returned"
    );
}

// L1018 Pull headers correct hash
#[tokio::test]
pub async fn pull_headers_correct_hash() {
    let setup = setup::client::default().await;

    sleep(Duration::from_secs(10)).await; // wait for the server to produce some blocks

    let tip_header = setup.client.tip().await;
    let headers = setup
        .client
        .pull_headers(
            &[setup.client.get_genesis_block_hash().await],
            tip_header.hash(),
        )
        .await
        .unwrap();
    let hashes: Vec<Hash> = headers.iter().map(|x| x.hash()).collect();

    let hashes_from_logs = setup.server.logger.get_created_blocks_hashes();
    assert!(is_long_prefix(&hashes, &hashes_from_logs));
}

// L1019 Pull headers incorrect hash
#[tokio::test]
pub async fn pull_headers_incorrect_hash() {
    let setup = setup::client::default().await;
    assert_eq!(
        MockClientError::InvalidRequest("not found (block not found)".into()),
        setup
            .client
            .pull_headers(&[], TestGen::hash())
            .await
            .err()
            .unwrap()
    );
}

// L1019A Pull headers empty hash
#[tokio::test]
#[ignore]
pub async fn pull_headers_empty_start_hash() {
    let setup = setup::client::default().await;

    sleep(Duration::from_secs(10)).await; // wait for the server to produce some blocks

    let tip_header = setup.client.tip().await;
    let headers = setup
        .client
        .pull_headers(&[], tip_header.hash())
        .await
        .unwrap();
    let hashes: Vec<Hash> = headers.iter().map(|x| x.hash()).collect();

    let hashes_from_logs = setup.server.logger.get_created_blocks_hashes();
    assert!(is_long_prefix(&hashes_from_logs, &hashes));
}

// L1020 Push headers incorrect header
#[tokio::test]
pub async fn push_headers() {
    let setup = setup::client::default().await;
    let tip_header = setup.client.tip().await;
    let stake_pool = StakePoolBuilder::new().build();

    let time_era = TimeEra::new(
        0u64.into(),
        Epoch(0u32),
        setup
            .config
            .block0_configuration()
            .blockchain_configuration
            .slots_per_epoch
            .into(),
    );

    let block = GenesisPraosBlockBuilder::new()
        .with_parent(&tip_header)
        .build(&stake_pool, &time_era);

    assert!(setup.client.push_headers(block.header).await.is_ok());
}

// L1020 Push headers incorrect header
#[tokio::test]
pub async fn upload_block_incompatible_protocol() {
    let setup = setup::client::default().await;
    let tip_header = setup.client.tip().await;
    let stake_pool = StakePoolBuilder::new().build();

    let time_era = TimeEra::new(
        0u64.into(),
        Epoch(0u32),
        setup
            .config
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
            "invalid request data (The block header verification failed: The block Version is incompatible with LeaderSelection.)".into() 
        ),
        setup.client.upload_blocks(block.clone()).await.err().unwrap()
    );
}

// L1020 Push headers incorrect header
#[tokio::test]
pub async fn upload_block_nonexisting_stake_pool() {
    let setup = setup::client::bootstrap(
        ConfigurationBuilder::new()
            .with_slot_duration(1)
            .with_block0_consensus(ConsensusVersion::GenesisPraos)
            .to_owned(),
    )
    .await;
    let tip_header = setup.client.tip().await;
    let stake_pool = StakePoolBuilder::new().build();

    let time_era = TimeEra::new(
        0u64.into(),
        Epoch(0u32),
        setup
            .config
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
            "invalid request data (The block header verification failed: Invalid block message)"
                .into()
        ),
        setup
            .client
            .upload_blocks(block.clone())
            .await
            .err()
            .unwrap()
    );
}

// L1020 Get fragments
#[tokio::test]
pub async fn get_fragments() {
    let mut sender = startup::create_new_account_address();
    let receiver = startup::create_new_account_address();
    let config = ConfigurationBuilder::new()
        .with_slot_duration(4)
        .with_funds(vec![InitialUTxO {
            address: sender.address(),
            value: 100.into(),
        }])
        .to_owned();

    let setup = setup::client::bootstrap(config).await;
    let output_value = 1u64;
    let jcli: JCli = Default::default();
    let transaction = sender
        .transaction_to(
            &setup.server.genesis_block_hash(),
            &setup.server.fees(),
            receiver.address(),
            output_value.into(),
        )
        .unwrap()
        .encode();

    let fragment_id = jcli
        .fragment_sender(&setup.server)
        .send(&transaction)
        .assert_in_block();
    println!("{:?}", setup.client.get_fragments(vec![fragment_id]).await);
}

// L1021 PullBlocks correct hashes
#[tokio::test]
pub async fn pull_blocks_correct_hashes_all_blocks() {
    let setup = setup::client::default().await;
    sleep(Duration::from_secs(10)).await; // wait for the server to produce some blocks

    let genesis_block_hash = Hash::from_str(setup.config.genesis_block_hash()).unwrap();
    let blocks = setup
        .client
        .pull_blocks(&[genesis_block_hash], setup.client.tip().await.id())
        .await
        .unwrap();

    let blocks_hashes: Vec<Hash> = blocks.iter().map(|x| x.header.hash()).collect();
    let block_hashes_from_logs = setup.server.logger.get_created_blocks_hashes();
    assert!(is_long_prefix(&block_hashes_from_logs, &blocks_hashes));
}

// L1022 PullBlocks correct hashes
#[tokio::test]
pub async fn pull_blocks_correct_hashes_partial() {
    let setup = setup::client::default().await;
    while setup.client.tip().await.chain_length() < 10.into() {
        tokio::time::sleep(CLIENT_RETRY_WAIT).await;
    }

    let block_hashes_from_logs = setup.server.logger.get_created_blocks_hashes();
    let start = 2;
    let end = 8;
    let expected_hashes = block_hashes_from_logs[start..end].to_vec();

    let blocks = setup
        .client
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
    let setup = setup::client::default().await;
    while setup.client.tip().await.chain_length() < 10.into() {
        tokio::time::sleep(CLIENT_RETRY_WAIT).await;
    }

    let block_hashes_from_logs = setup.server.logger.get_created_blocks_hashes();
    let start = 2;
    let end = 8;
    let expected_hashes = block_hashes_from_logs[start..end].to_vec();

    let result = setup
        .client
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
    let setup = setup::client::default().await;
    let from = TestGen::hash();
    let to = TestGen::hash();
    let result = setup.client.pull_blocks(&[from], to).await;
    assert!(result.is_err());
}
