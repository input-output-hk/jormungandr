use assert_fs::TempDir;
use chain_impl_mockchain::{
    block::{builder, BlockDate, BlockVersion, Contents},
    chaintypes::ConsensusType,
    header::HeaderBuilder,
    testing::TestGen,
};
use jormungandr_testing_utils::testing::{
    adversary::process::AdversaryNodeBuilder,
    jormungandr::{ConfigurationBuilder, Starter},
    startup,
};

#[test]
fn bft_block_with_incorrect_hash() {
    block_with_incorrect_hash(ConsensusType::Bft);
}

#[test]
fn genesis_block_with_incorrect_hash() {
    block_with_incorrect_hash(ConsensusType::GenesisPraos);
}

/// Ensures that blocks with an incorrect content hash are rejected by a BFT leader node
fn block_with_incorrect_hash(consensus: ConsensusType) {
    let temp_dir = TempDir::new().unwrap();
    let keys = startup::create_new_key_pair();

    let node_params = ConfigurationBuilder::default()
        .with_block0_consensus(consensus)
        .with_slot_duration(10)
        .with_leader_key_pair(keys.clone())
        .build(&temp_dir);

    let block0 = node_params.block0_configuration().to_block();

    let jormungandr = Starter::default().config(node_params).start().unwrap();

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
