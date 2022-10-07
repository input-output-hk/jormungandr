use assert_fs::TempDir;
use chain_impl_mockchain::{
    block::{builder, BlockDate, BlockVersion, Contents},
    chaintypes::ConsensusType,
    header::HeaderBuilder,
    testing::TestGen,
};
use jormungandr_automation::{
    jormungandr::{
        Block0ConfigurationBuilder, JormungandrBootstrapper, NodeConfigBuilder, SecretModelFactory,
    },
    testing::keys,
};
use loki::process::AdversaryNodeBuilder;

#[test]
fn bft_block_with_incorrect_hash() {
    block_with_incorrect_hash(ConsensusType::Bft);
}

#[test]
fn genesis_block_with_incorrect_hash() {
    block_with_incorrect_hash(ConsensusType::GenesisPraos);
}

#[test]
fn bft_block0_with_incorrect_hash() {
    block0_with_incorrect_hash(ConsensusType::Bft);
}

#[test]
fn genesis_praos_block0_with_incorrect_hash() {
    block0_with_incorrect_hash(ConsensusType::GenesisPraos)
}

/// Ensures that blocks with an incorrect content hash are rejected by a BFT leader node
fn block_with_incorrect_hash(consensus: ConsensusType) {
    let temp_dir = TempDir::new().unwrap();
    let keys = keys::create_new_key_pair();

    let node_params = Block0ConfigurationBuilder::default()
        .with_block0_consensus(consensus)
        .with_slot_duration(10.try_into().unwrap())
        .with_leader_key_pair(&keys)
        .build();

    let block0 = node_params.to_block();

    let jormungandr = JormungandrBootstrapper::default()
        .with_secret(SecretModelFactory::bft(keys.signing_key()))
        .with_block0_configuration(node_params)
        .start(temp_dir)
        .unwrap();

    let contents = Contents::empty();
    let content_size = contents.compute_hash_size().1;

    let block = builder(BlockVersion::Ed25519Signed, contents, |_| {
        Ok::<_, ()>({
            HeaderBuilder::new_raw(BlockVersion::Ed25519Signed, &TestGen::hash(), content_size)
                .set_parent(&block0.header().id(), 1.into())
                .set_date(BlockDate {
                    epoch: 0,
                    slot_id: 1,
                })
                .into_bft_builder()
                .unwrap()
                .sign_using(keys.0.private_key())
                .generalize()
        })
    })
    .unwrap();

    assert!(AdversaryNodeBuilder::new(block0)
        .build()
        .send_block_to_peer(jormungandr.address(), block)
        .is_err());
}

/// Ensures that the genesis block fetched during bootstrapping is the requested one.
fn block0_with_incorrect_hash(consensus: ConsensusType) {
    let block0 = Block0ConfigurationBuilder::default()
        .with_slot_duration(10.try_into().unwrap())
        .with_block0_consensus(consensus)
        .build()
        .to_block();

    let adversary = AdversaryNodeBuilder::new(block0)
        .with_protocol_version(consensus.into())
        .with_invalid_block0_hash()
        .with_server_enabled()
        .build();

    let passive_temp_dir = TempDir::new().unwrap();

    let passive_params = NodeConfigBuilder::default()
        .with_trusted_peers(vec![adversary.to_trusted_peer()])
        .build();

    JormungandrBootstrapper::default()
        .passive()
        .with_block0_hash(adversary.genesis_block_hash())
        .with_node_config(passive_params)
        .into_starter(passive_temp_dir)
        .unwrap()
        .start_with_fail_in_logs("failed to download block")
        .unwrap();
}
