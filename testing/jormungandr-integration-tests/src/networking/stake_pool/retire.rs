use hersir::{
    builder::{NetworkBuilder, Node, Topology},
    config::{BlockchainBuilder, SpawnParams, WalletTemplateBuilder},
};
use jormungandr_automation::{jormungandr::explorer::configuration::ExplorerParams, testing::time};
use jormungandr_lib::interfaces::BlockDate;
use thor::FragmentSender;
const LEADER_1: &str = "Leader_1";
const LEADER_2: &str = "Leader_2";
const LEADER_3: &str = "Leader_3";
const LEADER_4: &str = "Leader_4";

const ALICE: &str = "ALICE";
const BOB: &str = "BOB";
const CLARICE: &str = "CLARICE";
const DAVID: &str = "DAVID";

// FIX: there's a bug in our current handling of stake pool retirement
// re-enable this test once we fix that
#[test]
#[ignore]
pub fn retire_stake_pool_explorer() {
    let wait_epoch = 0;
    let wait_slot_id = 30;
    let mut controller = NetworkBuilder::default()
        .topology(
            Topology::default()
                .with_node(Node::new(LEADER_1))
                .with_node(Node::new(LEADER_2).with_trusted_peer(LEADER_1))
                .with_node(Node::new(LEADER_3).with_trusted_peer(LEADER_1))
                .with_node(Node::new(LEADER_4).with_trusted_peer(LEADER_1)),
        )
        .blockchain_config(
            BlockchainBuilder::default()
                .slots_per_epoch(60)
                .slot_duration(2)
                .leaders(vec![LEADER_1, LEADER_2, LEADER_3, LEADER_4])
                .build(),
        )
        .wallet_template(
            WalletTemplateBuilder::new(ALICE)
                .with(2_000_000_000)
                .delegated_to(LEADER_1)
                .build(),
        )
        .wallet_template(
            WalletTemplateBuilder::new(BOB)
                .with(2_000_000_000)
                .delegated_to(LEADER_2)
                .build(),
        )
        .wallet_template(
            WalletTemplateBuilder::new(CLARICE)
                .with(2_000_000_000)
                .delegated_to(LEADER_3)
                .build(),
        )
        .wallet_template(
            WalletTemplateBuilder::new(DAVID)
                .with(2_000_000_000)
                .delegated_to(LEADER_4)
                .build(),
        )
        .build()
        .unwrap();

    let leader_1 = controller
        .spawn(SpawnParams::new(LEADER_1).in_memory())
        .unwrap();
    let _leader_2 = controller
        .spawn(SpawnParams::new(LEADER_2).in_memory())
        .unwrap();
    let leader_3 = controller
        .spawn(SpawnParams::new(LEADER_3).in_memory())
        .unwrap();
    let _leader_4 = controller
        .spawn(SpawnParams::new(LEADER_4).in_memory())
        .unwrap();

    time::wait_for_date(BlockDate::new(wait_epoch, wait_slot_id), leader_1.rest());

    let explorer_process = leader_1.explorer(ExplorerParams::default()).unwrap();
    let explorer = explorer_process.client();
    let stake_pool_3 = controller.stake_pool(LEADER_3).unwrap().clone();

    let stake_pool_state_before = explorer
        .stake_pool(stake_pool_3.info().to_id().to_string(), 0)
        .unwrap();
    assert!(
        stake_pool_state_before
            .data
            .unwrap()
            .stake_pool
            .retirement
            .is_none(),
        "retirement field in explorer should be empty",
    );

    let mut david = controller.controlled_wallet(DAVID).unwrap();
    let mut spo_3 = stake_pool_3.owner().clone();

    let fragment_sender = FragmentSender::from(&controller.settings().block0);

    fragment_sender
        .send_transaction(&mut david, &spo_3, &leader_1, 100.into())
        .unwrap();

    fragment_sender
        .send_pool_retire(&mut spo_3, &stake_pool_3, &leader_1)
        .unwrap();

    time::wait_for_date(BlockDate::new(1, 30), leader_1.rest());

    let created_block_count = leader_3.logger.get_created_blocks_hashes().len();
    let start_time_no_block = std::time::SystemTime::now();

    // proof 1: explorer shows as retired
    let stake_pool_state_after = explorer
        .stake_pool(stake_pool_3.id().to_string(), 0)
        .unwrap();
    assert!(
        stake_pool_state_after
            .data
            .unwrap()
            .stake_pool
            .retirement
            .is_none(),
        "retirement field in explorer should not be empty",
    );

    // proof 2: minted block count not increased
    let created_blocks_count_after_retire = leader_3.logger.get_created_blocks_hashes().len();
    assert!(
        created_blocks_count_after_retire == created_block_count,
        "after retirement there are no new block minted",
    );

    //proof 3: no more minted blocks hashes in logs
    std::thread::sleep(std::time::Duration::from_secs(10));
    assert!(
        leader_3
            .logger
            .get_created_blocks_hashes_after(start_time_no_block.into())
            .is_empty(),
        "leader 3 should not create any block after retirement",
    );
}
