use crate::common::{
    jcli_wrapper,
    jormungandr::{ConfigurationBuilder, Starter},
    startup,
    transaction_utils::TransactionHash,
};
use crate::mock::{
    client::MockClientError,
    testing::{setup::bootstrap_node, setup::Config},
};
use chain_core::property::FromStr;
use chain_impl_mockchain::{
    block::Header,
    chaintypes::ConsensusVersion,
    key::Hash,
    testing::builders::{GenesisPraosBlockBuilder, StakePoolBuilder},
};
use chain_time::{Epoch, TimeEra};
use jormungandr_lib::interfaces::InitialUTxO;

fn fake_hash() -> Hash {
    Hash::from_str("efe2d4e5c4ad84b8e67e7b5676fff41cad5902a60b8cb6f072f42d7c7d26c944").unwrap()
}

// L1001 Handshake sanity
#[tokio::test]
pub async fn handshake_sanity() {
    let (_server, config) = bootstrap_node();
    let client = Config::attach_to_local_node(config.get_p2p_listen_port()).client();
    let handshake_response = client.handshake().await;

    assert_eq!(
        *config.genesis_block_hash(),
        hex::encode(handshake_response.block0),
        "Genesis Block"
    );
    assert_eq!(handshake_response.version, 1, "Protocol version");
}

// L1006 Tip request
#[tokio::test]
pub async fn tip_request() {
    let (server, config) = bootstrap_node();
    let client = Config::attach_to_local_node(config.get_p2p_listen_port()).client();

    let tip_header = client.tip().await;
    let block_hashes = server.logger.get_created_blocks_hashes();

    assert_eq!(*block_hashes.last().unwrap(), tip_header.hash());
}

// L1009 GetHeaders correct hash
#[tokio::test]
pub async fn get_headers_correct_hash() {
    let (server, config) = bootstrap_node();
    let client = Config::attach_to_local_node(config.get_p2p_listen_port()).client();

    let block_hashes = server.logger.get_created_blocks_hashes();
    let headers: Vec<Header> = client
        .headers(&block_hashes)
        .await
        .expect("unexpected error returned");
    let headers_hashes: Vec<Hash> = headers.iter().map(|x| x.hash()).collect();
    assert_eq!(block_hashes, headers_hashes);
}

// L1010 GetHeaders incorrect hash
#[tokio::test]
pub async fn get_headers_incorrect_hash() {
    let (_server, config) = bootstrap_node();
    let client = Config::attach_to_local_node(config.get_p2p_listen_port()).client();
    let fake_hash = fake_hash();
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
    let (_server, config) = bootstrap_node();
    let client = Config::attach_to_local_node(config.get_p2p_listen_port()).client();

    let tip = client.tip().await;
    assert!(client.get_blocks(&vec![tip.hash()]).await.is_ok());
}

// L1012 GetBlocks incorrect hash
#[tokio::test]
pub async fn get_blocks_incorrect_hash() {
    let (_server, config) = bootstrap_node();
    let client = Config::attach_to_local_node(config.get_p2p_listen_port()).client();
    let fake_hash = fake_hash();
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
    let (server, config) = bootstrap_node();
    let client = Config::attach_to_local_node(config.get_p2p_listen_port()).client();
    let blocks = client
        .pull_blocks_to_tip(Hash::from_str(config.genesis_block_hash()).unwrap())
        .await
        .unwrap();

    let blocks_hashes: Vec<Hash> = blocks.iter().map(|x| x.header.hash()).collect();

    let block_hashes_from_logs = server.logger.get_created_blocks_hashes();
    assert_eq!(block_hashes_from_logs, blocks_hashes);
}

// L1014 PullBlocksToTip incorrect hash
#[tokio::test]
pub async fn pull_blocks_to_tip_incorrect_hash() {
    let (server, config) = bootstrap_node();
    let client = Config::attach_to_local_node(config.get_p2p_listen_port()).client();
    let blocks = client.pull_blocks_to_tip(fake_hash()).await.unwrap();
    let blocks_hashes: Vec<Hash> = blocks.iter().map(|x| x.header.hash()).collect();

    let hashes_from_logs = server.logger.get_created_blocks_hashes();

    assert_eq!(
        hashes_from_logs, blocks_hashes,
        "If requested hash doesn't point to any block, all blocks should be returned"
    );
}

// L1018 Pull headers correct hash
#[tokio::test]
pub async fn pull_headers_correct_hash() {
    let (server, config) = bootstrap_node();
    let client = Config::attach_to_local_node(config.get_p2p_listen_port()).client();
    let tip_header = client.tip().await;
    let headers = client
        .pull_headers(&[client.get_genesis_block_hash().await], tip_header.hash())
        .await
        .unwrap();
    let hashes: Vec<Hash> = headers.iter().map(|x| x.hash()).collect();

    let hashes_from_logs = server.logger.get_created_blocks_hashes();
    assert_eq!(hashes, hashes_from_logs);
}

// L1019 Pull headers incorrect hash
#[tokio::test]
pub async fn pull_headers_incorrect_hash() {
    let (_server, config) = bootstrap_node();
    let client = Config::attach_to_local_node(config.get_p2p_listen_port()).client();
    assert_eq!(
        MockClientError::InvalidRequest(format!("not found (block not found)")),
        client.pull_headers(&[], fake_hash()).await.err().unwrap()
    );
}

// L1019A Pull headers empty hash
#[tokio::test]
pub async fn pull_headers_empty_start_hash() {
    let (server, config) = bootstrap_node();
    let client = Config::attach_to_local_node(config.get_p2p_listen_port()).client();
    let tip_header = client.tip().await;
    let headers = client.pull_headers(&[], tip_header.hash()).await.unwrap();
    let hashes: Vec<Hash> = headers.iter().map(|x| x.hash()).collect();

    let hashes_from_logs = server.logger.get_created_blocks_hashes();
    assert_eq!(hashes, hashes_from_logs);
}

// L1020 Push headers incorrect header
#[tokio::test]
pub async fn push_headers() {
    let (_server, config) = bootstrap_node();
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
    let config = ConfigurationBuilder::new().with_slot_duration(4).build();
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
    let config = ConfigurationBuilder::new()
        .with_slot_duration(4)
        .with_block0_consensus(ConsensusVersion::GenesisPraos)
        .build();
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
    let mut sender = startup::create_new_account_address();
    let receiver = startup::create_new_account_address();
    let output_value = 1u64;
    let config = ConfigurationBuilder::new()
        .with_slot_duration(4)
        .with_funds(vec![InitialUTxO {
            address: sender.address(),
            value: 100.into(),
        }])
        .build();

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

    let fragment_id = jcli_wrapper::assert_transaction_in_block(&transaction, &server);
    let client = Config::attach_to_local_node(config.get_p2p_listen_port()).client();
    println!(
        "{:?}",
        client.get_fragments(vec![fragment_id.into_hash()]).await
    );
}
