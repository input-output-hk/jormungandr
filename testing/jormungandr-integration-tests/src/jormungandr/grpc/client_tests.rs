use super::setup;
use chain_core::property::{Block as _, FromStr};
use chain_crypto::{Ed25519, PublicKey, Signature, Verification};
use chain_impl_mockchain::{
    block::{BlockDate, Header},
    chaintypes::ConsensusVersion,
    key::Hash,
    testing::{
        builders::{GenesisPraosBlockBuilder, StakePoolBuilder},
        TestGen,
    },
};
use chain_time::{Epoch, TimeEra};
use jormungandr_automation::{
    jcli::JCli,
    jormungandr::{grpc::client::MockClientError, ConfigurationBuilder},
};
use jormungandr_lib::interfaces::InitialUTxO;
use rand::Rng;
use std::time::Duration;
use thor::TransactionHash;

const CHAIN_GROWTH_TIMEOUT: Duration = Duration::from_secs(60);

// check that affix is a long enough (at least half the size) prefix of word
fn is_long_prefix<T: PartialEq>(word: &[T], affix: &[T]) -> bool {
    if word.len() < affix.len() || affix.len() * 2 < word.len() {
        return false;
    }
    affix.iter().zip(word.iter()).all(|(x, y)| x == y)
}

// L1001 Handshake sanity
#[test]
pub fn handshake_sanity() {
    let setup = setup::client::default();
    let mut auth_nonce = [0u8; 32];
    rand::thread_rng().fill(&mut auth_nonce[..]);
    let handshake_response = setup.client.handshake(&auth_nonce);

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
#[test]
pub fn tip_request() {
    let setup = setup::client::bootstrap(
        ConfigurationBuilder::new()
            .with_slot_duration(15)
            .to_owned(),
    );

    setup
        .client
        .wait_for_chain_length(1.into(), CHAIN_GROWTH_TIMEOUT);

    let tip_header = setup.client.tip();
    let block_hashes = setup.server.logger.get_created_blocks_hashes();

    if *block_hashes.last().unwrap() != tip_header.hash() {
        //if the server produces another block compare with second last
        assert_eq!(block_hashes[block_hashes.len() - 2], tip_header.hash());
    }
}

// L1009 GetHeaders correct hash
#[test]
pub fn get_headers_correct_hash() {
    let setup = setup::client::default();

    std::thread::sleep(Duration::from_secs(10)); // wait for the server to produce some blocks

    let block_hashes = setup.server.logger.get_created_blocks_hashes();
    let headers: Vec<Header> = setup
        .client
        .headers(&block_hashes)
        .expect("unexpected error returned");
    let headers_hashes: Vec<Hash> = headers.iter().map(|x| x.hash()).collect();
    assert!(
        is_long_prefix(&block_hashes, &headers_hashes),
        "server blocks: {:?} | client blocks: {:?}",
        block_hashes,
        headers_hashes,
    );
}

// L1010 GetHeaders incorrect hash
#[test]
pub fn get_headers_incorrect_hash() {
    let setup = setup::client::default();
    let fake_hash: Hash = TestGen::hash();
    assert_eq!(
        MockClientError::InvalidRequest(format!(
            "not found (block {} is not known to this node)",
            fake_hash
        )),
        setup.client.headers(&[fake_hash]).err().unwrap(),
        "wrong error"
    );
}

// L1011 GetBlocks correct hash
#[test]
pub fn get_blocks_correct_hash() {
    let setup = setup::client::default();

    let tip = setup.client.tip();
    assert!(setup.client.get_blocks(&[tip.hash()]).is_ok());
}

// L1012 GetBlocks incorrect hash
#[test]
pub fn get_blocks_incorrect_hash() {
    let setup = setup::client::default();
    let fake_hash: Hash = TestGen::hash();
    assert_eq!(
        MockClientError::InvalidRequest(format!(
            "not found (block {} is not known to this node)",
            fake_hash
        )),
        setup.client.headers(&[fake_hash]).err().unwrap(),
        "wrong error"
    );
}

// L1013 PullBlocksToTip correct hash
#[test]
pub fn pull_blocks_to_tip_correct_hash() {
    let setup = setup::client::default();

    std::thread::sleep(Duration::from_secs(10)); // wait for the server to produce some blocks

    let blocks = setup
        .client
        .pull_blocks_to_tip(Hash::from_str(setup.config.genesis_block_hash()).unwrap())
        .unwrap();

    let blocks_hashes: Vec<Hash> = blocks.iter().map(|x| x.header().hash()).collect();

    let block_hashes_from_logs = setup.server.logger.get_created_blocks_hashes();
    assert!(
        is_long_prefix(&block_hashes_from_logs, &blocks_hashes),
        "server blocks: {:?} | client blocks: {:?}",
        block_hashes_from_logs,
        blocks_hashes
    );
}

#[test]
pub fn pull_range_invalid_params() {
    let setup = setup::client::default();

    std::thread::sleep(Duration::from_secs(10)); // wait for the server to produce some blocks
    let gen_hash = Hash::from_str(setup.config.genesis_block_hash()).unwrap();
    let client = setup.client;
    let tip_hash = client.tip().hash();
    let fake_hash = TestGen::hash();
    let error = MockClientError::InvalidRequest(
        "not found (Could not find a known block in `from`)".into(),
    );

    let invalid_params: [(&[Hash], Hash); 3] = [
        (&[], tip_hash),
        (&[fake_hash], tip_hash),
        (&[gen_hash], fake_hash),
    ];
    for (from, to) in invalid_params.iter() {
        assert_eq!(error, client.pull_headers(from, *to).err().unwrap());
        assert_eq!(error, client.pull_blocks(from, *to).err().unwrap());
    }
    assert_eq!(error, client.pull_blocks_to_tip(fake_hash).err().unwrap());
}

// L1018 Pull headers correct hash
#[test]
pub fn pull_headers_correct_hash() {
    let setup = setup::client::default();

    std::thread::sleep(Duration::from_secs(10)); // wait for the server to produce some blocks

    let tip_header = setup.client.tip();
    let headers = setup
        .client
        .pull_headers(&[setup.client.get_genesis_block_hash()], tip_header.hash())
        .unwrap();
    let hashes: Vec<Hash> = headers.iter().map(|x| x.hash()).collect();

    let hashes_from_logs = setup.server.logger.get_created_blocks_hashes();
    assert!(
        is_long_prefix(&hashes_from_logs, &hashes),
        "server blocks: {:?} | client blocks: {:?}",
        hashes_from_logs,
        hashes,
    );
}

// L1020 Push headers incorrect header
#[test]
pub fn push_headers() {
    let setup = setup::client::default();
    let tip_header = setup.client.tip();
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

    assert!(setup.client.push_headers(block.header().clone()).is_ok());
}

// L1020 Push headers incorrect header
#[test]
pub fn upload_block_incompatible_protocol() {
    let setup = setup::client::default();
    let tip_header = setup.client.tip();
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
            "invalid request data (the block header verification failed)".into()
        ),
        setup.client.upload_blocks(block).err().unwrap()
    );
}

// L1020 Push headers incorrect header
#[test]
pub fn upload_block_nonexisting_stake_pool() {
    let setup = setup::client::bootstrap(
        ConfigurationBuilder::new()
            .with_block0_consensus(ConsensusVersion::GenesisPraos)
            .to_owned(),
    );
    let tip_header = setup.client.tip();
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
            "invalid request data (the block header verification failed)".into()
        ),
        setup.client.upload_blocks(block).err().unwrap()
    );
}

// L1020 Get fragments
#[test]
pub fn get_fragments() {
    let sender = thor::Wallet::default();
    let receiver = thor::Wallet::default();
    let config = ConfigurationBuilder::new()
        .with_slot_duration(4)
        .with_funds(vec![InitialUTxO {
            address: sender.address(),
            value: 100.into(),
        }])
        .to_owned();

    let setup = setup::client::bootstrap(config);
    let output_value = 1u64;
    let jcli: JCli = Default::default();
    let transaction = thor::FragmentBuilder::new(
        &setup.server.genesis_block_hash(),
        &setup.server.fees(),
        BlockDate::first().next_epoch(),
    )
    .transaction(&sender, receiver.address(), output_value.into())
    .unwrap()
    .encode();

    let fragment_id = jcli
        .fragment_sender(&setup.server)
        .send(&transaction)
        .assert_in_block();
    println!("{:?}", setup.client.get_fragments(vec![fragment_id]));
}

// L1021 PullBlocks correct hashes
#[test]
pub fn pull_blocks_correct_hashes_all_blocks() {
    let setup = setup::client::default();
    std::thread::sleep(Duration::from_secs(10)); // wait for the server to produce some blocks

    let genesis_block_hash = Hash::from_str(setup.config.genesis_block_hash()).unwrap();
    let blocks = setup
        .client
        .pull_blocks(&[genesis_block_hash], setup.client.tip().id())
        .unwrap();

    let blocks_hashes: Vec<Hash> = blocks.iter().map(|x| x.header().hash()).collect();
    let block_hashes_from_logs = setup.server.logger.get_created_blocks_hashes();
    assert!(
        is_long_prefix(&block_hashes_from_logs, &blocks_hashes),
        "server blocks: {:?} | client blocks: {:?}",
        block_hashes_from_logs,
        blocks_hashes
    );
}

// L1022 PullBlocks correct hashes
#[test]
pub fn pull_blocks_correct_hashes_partial() {
    let setup = setup::client::default();
    setup
        .client
        .wait_for_chain_length(10.into(), CHAIN_GROWTH_TIMEOUT);

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
        .unwrap();

    let blocks_hashes: Vec<Hash> = blocks.iter().map(|x| x.header().hash()).collect();

    assert_eq!(&expected_hashes[1..], &blocks_hashes);
}

// L1023 PullBlocks to and from in wrong order
#[test]
pub fn pull_blocks_hashes_wrong_order() {
    let setup = setup::client::default();

    setup
        .client
        .wait_for_chain_length(10.into(), CHAIN_GROWTH_TIMEOUT);

    let block_hashes_from_logs = setup.server.logger.get_created_blocks_hashes();
    let start = 2;
    let end = 8;
    let expected_hashes = block_hashes_from_logs[start..end].to_vec();

    let result = setup.client.pull_blocks(
        &[expected_hashes.last().copied().unwrap()],
        expected_hashes[0],
    );

    assert!(result.is_err());
}

#[test]
pub fn test_watch_block_subscription_blocks_are_in_logs() {
    use std::collections::HashSet;

    let setup = setup::client::default();

    let watch_client = setup.watch_client;

    let (sender, receiver) = std::sync::mpsc::channel();

    watch_client.block_subscription(sender);

    let mut ids = HashSet::new();

    const BLOCKS_TO_TEST: usize = 20;

    while let Ok(block) = receiver.recv() {
        assert!(ids.insert(block.unwrap().id()));

        if ids.len() == BLOCKS_TO_TEST {
            break;
        }
    }

    // wait a bit in case are not flushed yet
    std::thread::sleep(Duration::from_millis(250));

    let block_hashes_from_logs: HashSet<Hash> = setup
        .server
        .logger
        .get_created_blocks_hashes()
        .into_iter()
        .collect();

    assert!(dbg!(ids).is_subset(&dbg!(block_hashes_from_logs)));
}

#[test]
pub fn test_watch_tip_subscription_is_current_tip() {
    let setup = setup::client::bootstrap(
        ConfigurationBuilder::new()
            .with_slot_duration(3u8)
            .to_owned(),
    );
    let rest = setup.server.rest();
    let watch_client = setup.watch_client;

    let notif = watch_client.tip_subscription();

    let (watch_tip, cond) = &*notif;

    let mut iters_remaining: usize = 10;
    let mut guard = watch_tip.lock().unwrap();

    loop {
        println!("iter remaining: {}", iters_remaining);
        let header = &*guard;

        let rest_tip = rest.tip().unwrap();
        assert_eq!(header.as_ref().unwrap().id(), rest_tip.into_hash());

        if iters_remaining == 0 {
            // here we still hold the lock, it will be unlocked when dropping the stack
            return;
        } else {
            iters_remaining -= 1;

            guard = cond.wait(guard).unwrap();
        }
    }
}

#[test]
pub fn test_watch_sync_multiverse_full() {
    use std::collections::HashSet;

    let setup = setup::client::default();
    let watch_client = setup.watch_client;

    setup
        .client
        .wait_for_chain_length(15.into(), CHAIN_GROWTH_TIMEOUT);

    let blocks = watch_client.sync_multiverse(vec![]);

    let block_hashes_from_logs: HashSet<Hash> = setup
        .server
        .logger
        .get_created_blocks_hashes()
        .into_iter()
        .collect();

    let mut block_set: HashSet<Hash> = Default::default();
    let mut max_length = None;

    for block in &blocks {
        if !block_set.is_empty() {
            assert!(block_set.contains(&block.header().block_parent_hash()))
        }

        max_length.replace(block.header().chain_length());

        assert!(block_set.insert(block.id()));
    }

    // the genesis block is not in the logs
    let genesis = blocks[0].clone();
    block_set.remove(&genesis.id());

    assert!(block_set.is_subset(&block_hashes_from_logs));

    // There is a possibility of new blocks produced between `sync_multiverse` and
    // `get_created_blocks_hashes`, so here we assert they were produced after.  This is actually
    // not necessary for bft, because we are already checking the order by checking the parents,
    // but for genesis praos we need this to be sure that we are sending all the branches.
    //
    // This test currently uses bft, though, so the checks are mostly as a precaution.
    let rest = setup.server.rest();

    for hash in block_hashes_from_logs.difference(&block_set) {
        let missing_block = rest.block(hash).unwrap();

        assert!(missing_block.header().chain_length() > max_length.unwrap());
    }
}
