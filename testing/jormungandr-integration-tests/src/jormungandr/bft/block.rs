use assert_fs::TempDir;
use chain_impl_mockchain::{
    block::{builder, BlockDate, BlockVersion, Contents, ContentsBuilder},
    chaintypes::{ConsensusType, ConsensusVersion},
    fee::LinearFee,
    header::HeaderBuilder,
    testing::TestGen,
};
use jormungandr_lib::interfaces::SlotDuration;
use jormungandr_testing_utils::testing::{
    adversary::process::AdversaryNodeBuilder,
    jormungandr::{ConfigurationBuilder, Starter, StartupVerificationMode},
    network::{builder::NetworkBuilder, Blockchain, Node, SpawnParams, Topology},
    startup, FragmentBuilder,
};
use std::time::Duration;

#[test]
fn block_with_incorrect_hash() {
    let temp_dir = TempDir::new().unwrap();
    let keys = startup::create_new_key_pair();

    let node_params = ConfigurationBuilder::default()
        .with_block0_consensus(ConsensusType::Bft)
        .with_slot_duration(10)
        .with_leader_key_pair(keys.clone())
        .build(&temp_dir);

    let block0 = node_params.block0_configuration().to_block();

    let jormungandr = Starter::default().config(node_params).start().unwrap();

    jormungandr
        .wait_for_bootstrap(&StartupVerificationMode::Rest, Duration::from_secs(1))
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

    let mut adversary = AdversaryNodeBuilder::new(block0).build();

    assert!(adversary
        .send_block_to_peer(jormungandr.address(), block)
        .is_err());
}

#[test]
fn block_with_bad_signature() {
    // TODO
}

#[test]
fn block_with_wrong_leader() {
    const LEADER_1: &str = "Abbott";
    const LEADER_2: &str = "Costello";

    let mut blockchain_config = Blockchain::default();
    blockchain_config.set_consensus(ConsensusVersion::Bft);
    blockchain_config.set_slot_duration(SlotDuration::new(10).unwrap());
    blockchain_config.add_leader(LEADER_1);
    blockchain_config.add_leader(LEADER_2);

    let mut controller = NetworkBuilder::default()
        .blockchain_config(blockchain_config)
        .topology(
            Topology::default()
                .with_node(Node::new(LEADER_1))
                .with_node(Node::new(LEADER_2).with_trusted_peer(LEADER_1)),
        )
        .build()
        .unwrap();

    let leader = controller
        .spawn(SpawnParams::new(LEADER_1).leader())
        .unwrap();

    let block0 = leader.block0_configuration().to_block();

    let wrong_leader_block = builder(
        BlockVersion::Ed25519Signed,
        Contents::empty(),
        |hdr_builder| {
            Ok::<_, ()>({
                hdr_builder
                    .set_parent(&block0.header().id(), 1.into())
                    .set_date(BlockDate {
                        epoch: 0,
                        slot_id: 1,
                    })
                    .into_bft_builder()
                    .unwrap()
                    .sign_using(
                        &controller
                            .node_settings(LEADER_1)
                            .unwrap()
                            .secret
                            .bft
                            .as_ref()
                            .unwrap()
                            .signing_key
                            .clone()
                            .into_secret_key(),
                    )
                    .generalize()
            })
        },
    )
    .unwrap();

    let correct_leader_block = builder(
        BlockVersion::Ed25519Signed,
        Contents::empty(),
        |hdr_builder| {
            Ok::<_, ()>({
                hdr_builder
                    .set_parent(&block0.header().id(), 1.into())
                    .set_date(BlockDate {
                        epoch: 0,
                        slot_id: 1,
                    })
                    .into_bft_builder()
                    .unwrap()
                    .sign_using(
                        &controller
                            .node_settings(LEADER_2)
                            .unwrap()
                            .secret
                            .bft
                            .as_ref()
                            .unwrap()
                            .signing_key
                            .clone()
                            .into_secret_key(),
                    )
                    .generalize()
            })
        },
    )
    .unwrap();

    let mut adversary = AdversaryNodeBuilder::new(block0).build();

    assert!(adversary
        .send_block_to_peer(leader.address(), wrong_leader_block)
        .is_err());

    assert!(adversary
        .send_block_to_peer(leader.address(), correct_leader_block)
        .is_ok());
}

#[test]
fn block_with_nonexistent_leader() {
    let temp_dir = TempDir::new().unwrap();

    let node_params = ConfigurationBuilder::default()
        .with_block0_consensus(ConsensusType::Bft)
        .with_slot_duration(10)
        .build(&temp_dir);

    let block0 = node_params.block0_configuration().to_block();

    let jormungandr = Starter::default().config(node_params).start().unwrap();

    jormungandr
        .wait_for_bootstrap(&StartupVerificationMode::Rest, Duration::from_secs(1))
        .unwrap();

    let contents = Contents::empty();

    let block = builder(BlockVersion::Ed25519Signed, contents, |hdr_builder| {
        Ok::<_, ()>({
            hdr_builder
                .set_parent(&block0.header().id(), 1.into())
                .set_date(BlockDate {
                    epoch: 0,
                    slot_id: 1,
                })
                .into_bft_builder()
                .unwrap()
                .sign_using(startup::create_new_key_pair().0.private_key())
                .generalize()
        })
    })
    .unwrap();

    let mut adversary = AdversaryNodeBuilder::new(block0).build();

    assert!(adversary
        .send_block_to_peer(jormungandr.address(), block)
        .is_err());
}

#[test]
fn block_with_invalid_fragment() {
    let temp_dir = TempDir::new().unwrap();
    let keys = startup::create_new_key_pair();

    let node_params = ConfigurationBuilder::default()
        .with_block0_consensus(ConsensusType::Bft)
        .with_slot_duration(10)
        .with_leader_key_pair(keys.clone())
        .build(&temp_dir);

    let block0 = node_params.block0_configuration().to_block();

    let jormungandr = Starter::default().config(node_params).start().unwrap();

    jormungandr
        .wait_for_bootstrap(&StartupVerificationMode::Rest, Duration::from_secs(1))
        .unwrap();

    let mut contents_builder = ContentsBuilder::default();

    contents_builder.push(
        FragmentBuilder::new(
            &jormungandr.genesis_block_hash(),
            &LinearFee::new(0, 0, 0),
            BlockDate::first().next_epoch(),
        )
        .transaction(
            &startup::create_new_account_address(),
            startup::create_new_account_address().address(),
            42.into(),
        )
        .unwrap(),
    );

    let block = builder(
        BlockVersion::Ed25519Signed,
        contents_builder.into(),
        |hdr_builder| {
            Ok::<_, ()>({
                hdr_builder
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
        },
    )
    .unwrap();

    let mut adversary = AdversaryNodeBuilder::new(block0).build();

    assert!(adversary
        .send_block_to_peer(jormungandr.address(), block)
        .is_err());
}
