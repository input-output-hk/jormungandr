use assert_fs::TempDir;
use chain_impl_mockchain::{
    block::{BlockDate, ContentsBuilder},
    chaintypes::{ConsensusType, ConsensusVersion},
    fee::LinearFee,
};
use hersir::{
    builder::{NetworkBuilder, Node, Topology},
    config::{Blockchain, SpawnParams},
};
use jormungandr_automation::{
    jormungandr::{ConfigurationBuilder, Starter},
    testing::keys,
};
use jormungandr_lib::interfaces::SlotDuration;
use loki::{block::BlockBuilder, process::AdversaryNodeBuilder};
use thor::FragmentBuilder;

#[test]
/// Ensures that blocks with an incorrect signature are rejected by a BFT leader node
fn block_with_incorrect_signature() {
    let temp_dir = TempDir::new().unwrap();
    let keys = keys::create_new_key_pair();

    let node_params = ConfigurationBuilder::default()
        .with_block0_consensus(ConsensusType::Bft)
        .with_slot_duration(10)
        .with_leader_key_pair(keys.clone())
        .build(&temp_dir);

    let block0 = node_params.block0_configuration().to_block();

    let jormungandr = Starter::default().config(node_params).start().unwrap();

    let block = BlockBuilder::bft(
        BlockDate {
            epoch: 0,
            slot_id: 1,
        },
        block0.header().clone(),
    )
    .signing_key(keys.signing_key())
    .invalid_signature()
    .build();

    assert!(AdversaryNodeBuilder::new(block0)
        .build()
        .send_block_to_peer(jormungandr.address(), block)
        .is_err());
}

#[test]
/// Ensures that blocks signed by the wrong leader on a given timeslot are rejected by a BFT leader node
fn block_with_wrong_leader() {
    const LEADER_1: &str = "Abbott";
    const LEADER_2: &str = "Costello";

    let blockchain_config = Blockchain::default()
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

    let block0 = leader.block0_configuration().to_block();

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

    let node_params = ConfigurationBuilder::default()
        .with_block0_consensus(ConsensusType::Bft)
        .with_slot_duration(10)
        .build(&temp_dir);

    let block0 = node_params.block0_configuration().to_block();

    let jormungandr = Starter::default().config(node_params).start().unwrap();

    let block = BlockBuilder::bft(
        BlockDate {
            epoch: 0,
            slot_id: 1,
        },
        block0.header().clone(),
    )
    .build();

    assert!(AdversaryNodeBuilder::new(block0)
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

    let node_params = ConfigurationBuilder::default()
        .with_block0_consensus(ConsensusType::Bft)
        .with_slot_duration(10)
        .with_leader_key_pair(keys.clone())
        .build(&temp_dir);

    let block0 = node_params.block0_configuration().to_block();

    let jormungandr = Starter::default().config(node_params).start().unwrap();

    let mut contents_builder = ContentsBuilder::default();

    contents_builder.push(
        FragmentBuilder::new(
            &jormungandr.genesis_block_hash(),
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
