use crate::startup::SingleNodeTestBootstrapper;
use assert_fs::TempDir;
use chain_impl_mockchain::{
    block::{BlockDate, ContentsBuilder},
    chaintypes::{ConsensusType, ConsensusVersion},
    fee::LinearFee,
};
use hersir::{
    builder::{NetworkBuilder, Node, Topology},
    config::{BlockchainConfiguration, SpawnParams},
};
use jormungandr_automation::{
    jormungandr::{Block0ConfigurationBuilder, JormungandrBootstrapper},
    testing::{block0::Block0ConfigurationExtension, keys},
};
use jormungandr_lib::interfaces::SlotDuration;
use loki::{block::BlockBuilder, process::AdversaryNodeBuilder};
use thor::FragmentBuilder;

#[test]
/// Ensures that blocks with an incorrect signature are rejected by a BFT leader node
fn block_with_incorrect_signature() {
    let temp_dir = TempDir::new().unwrap();
    let keys = keys::create_new_key_pair();

    let block0 = Block0ConfigurationBuilder::default()
        .with_block0_consensus(ConsensusType::Bft)
        .with_slot_duration(SlotDuration::new(10).unwrap())
        .with_leader_key_pair(&keys)
        .build();

    let jormungandr = JormungandrBootstrapper::default()
        .with_leader_key(&keys)
        .with_block0_configuration(block0.clone())
        .start(temp_dir)
        .unwrap();

    let block = BlockBuilder::bft(
        BlockDate {
            epoch: 0,
            slot_id: 1,
        },
        block0.to_block().header().clone(),
    )
    .signing_key(keys.signing_key())
    .invalid_signature()
    .build();

    assert!(AdversaryNodeBuilder::new(block0.to_block())
        .build()
        .send_block_to_peer(jormungandr.address(), block)
        .is_err());
}

#[test]
/// Ensures that blocks signed by the wrong leader on a given timeslot are rejected by a BFT leader node
fn block_with_wrong_leader() {
    const LEADER_1: &str = "Abbott";
    const LEADER_2: &str = "Costello";

    let blockchain_config = BlockchainConfiguration::default()
        .with_consensus(ConsensusVersion::Bft)
        .with_slot_duration(SlotDuration::new(10).unwrap())
        .with_leader(LEADER_1)
        .with_leader(LEADER_2);

    let mut controller = NetworkBuilder::default()
        .blockchain_config(blockchain_config)
        .topology(
            Topology::default()
                .with_node(Node::new(LEADER_1))
                .with_node(Node::new(LEADER_2)),
        )
        .build()
        .unwrap();

    let leader = controller
        .spawn(SpawnParams::new(LEADER_1).leader())
        .unwrap();

    let block0 = controller.settings().block0.to_block();

    let wrong_leader_block = BlockBuilder::bft(
        BlockDate {
            epoch: 0,
            slot_id: 1,
        },
        block0.header().clone(),
    )
    .signing_key(
        controller
            .node_settings(LEADER_1)
            .unwrap()
            .secret
            .bft
            .as_ref()
            .unwrap()
            .signing_key
            .clone(),
    )
    .build();

    let correct_leader_block = BlockBuilder::bft(
        BlockDate {
            epoch: 0,
            slot_id: 1,
        },
        block0.header().clone(),
    )
    .signing_key(
        controller
            .node_settings(LEADER_2)
            .unwrap()
            .secret
            .bft
            .as_ref()
            .unwrap()
            .signing_key
            .clone(),
    )
    .build();

    let mut adversary = AdversaryNodeBuilder::new(block0).build();

    assert!(adversary
        .send_block_to_peer(leader.address(), wrong_leader_block)
        .is_err());

    assert!(adversary
        .send_block_to_peer(leader.address(), correct_leader_block)
        .is_ok());
}

#[test]
/// Ensures that blocks signed by a non-existent leader are rejected by a BFT leader node
fn block_with_nonexistent_leader() {
    let temp_dir = TempDir::new().unwrap();

    let block0_configurator =
        Block0ConfigurationBuilder::default().with_slot_duration(SlotDuration::new(10).unwrap());

    let context = SingleNodeTestBootstrapper::default()
        .with_block0_config(block0_configurator)
        .as_bft_leader()
        .build();
    let jormungandr = context.start_node(temp_dir).unwrap();

    let block = BlockBuilder::bft(
        BlockDate {
            epoch: 0,
            slot_id: 1,
        },
        context.block0_config().to_block().header().clone(),
    )
    .build();

    assert!(AdversaryNodeBuilder::new(context.block0_config.to_block())
        .build()
        .send_block_to_peer(jormungandr.address(), block)
        .is_err());
}

#[test]
/// Ensures that blocks with an invalid fragment –in this case a transaction from a non-existent
/// wallet- are rejected by a BFT leader node
fn block_with_invalid_fragment() {
    let temp_dir = TempDir::new().unwrap();
    let keys = keys::create_new_key_pair();

    let block0_config = Block0ConfigurationBuilder::default()
        .with_block0_consensus(ConsensusType::Bft)
        .with_slot_duration(SlotDuration::new(10).unwrap())
        .with_leader_key_pair(&keys)
        .build();

    let block0 = block0_config.to_block();
    let jormungandr = JormungandrBootstrapper::default()
        .with_block0_configuration(block0_config.clone())
        .with_leader_key(&keys)
        .start(temp_dir)
        .unwrap();

    let mut contents_builder = ContentsBuilder::default();

    contents_builder.push(
        FragmentBuilder::new(
            &block0_config.to_block_hash(),
            &LinearFee::new(0, 0, 0),
            BlockDate::first().next_epoch(),
        )
        .transaction(
            &thor::Wallet::default(),
            thor::Wallet::default().address(),
            42.into(),
        )
        .unwrap(),
    );

    let block = BlockBuilder::bft(
        BlockDate {
            epoch: 0,
            slot_id: 1,
        },
        block0.header().clone(),
    )
    .signing_key(keys.signing_key())
    .contents(contents_builder.into())
    .build();

    assert!(AdversaryNodeBuilder::new(block0)
        .build()
        .send_block_to_peer(jormungandr.address(), block)
        .is_err());
}
