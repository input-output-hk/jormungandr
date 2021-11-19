use crate::{
    test::{utils, Result},
    Context, ScenarioResult,
};

use jormungandr_lib::interfaces::Explorer;
use jormungandr_testing_utils::testing::network::{LeadershipMode, PersistenceMode};
use jormungandr_testing_utils::testing::FragmentSender;
const LEADER_1: &str = "Leader_1";
const LEADER_2: &str = "Leader_2";
const LEADER_3: &str = "Leader_3";
const LEADER_4: &str = "Leader_4";
use function_name::named;

#[named]
pub fn retire_stake_pool_explorer(context: Context) -> Result<ScenarioResult> {
    let name = function_name!();
    let scenario_settings = prepare_scenario! {
        name,
        &mut context,
        topology [
            LEADER_1,
            LEADER_2 -> LEADER_1,
            LEADER_3 -> LEADER_1,
            LEADER_4 -> LEADER_1,
        ]
        blockchain {
            consensus = GenesisPraos,
            number_of_slots_per_epoch = 60,
            slot_duration = 1,
            leaders = [ LEADER_1 ],
            initials = [
                "account" "alice" with  2_000_000_000 delegates to LEADER_1,
                "account" "bob" with  2_000_000_000 delegates to LEADER_2,
                "account" "clarice" with  2_000_000_000 delegates to LEADER_3,
                "account" "david" with  2_000_000_000 delegates to LEADER_4,
            ],
        }
    };

    let mut controller = scenario_settings.build(context)?;

    let mut leader_1 = controller.spawn_node_custom(
        controller
            .new_spawn_params(LEADER_1)
            .leader()
            .in_memory()
            .explorer(Explorer { enabled: true }),
    )?;
    leader_1.wait_for_bootstrap()?;

    let mut leader_2 =
        controller.spawn_node(LEADER_2, LeadershipMode::Leader, PersistenceMode::InMemory)?;
    leader_2.wait_for_bootstrap()?;

    let mut leader_3 =
        controller.spawn_node(LEADER_3, LeadershipMode::Leader, PersistenceMode::InMemory)?;
    leader_3.wait_for_bootstrap()?;

    let mut leader_4 =
        controller.spawn_node(LEADER_4, LeadershipMode::Leader, PersistenceMode::InMemory)?;
    leader_4.wait_for_bootstrap()?;

    controller.monitor_nodes();

    std::thread::sleep(std::time::Duration::from_secs(30));

    let explorer = leader_1.explorer();
    let stake_pool_3 = controller.stake_pool(LEADER_3)?;

    let stake_pool_state_before =
        explorer.stake_pool(stake_pool_3.info().to_id().to_string(), 0)?;
    utils::assert(
        stake_pool_state_before
            .data
            .unwrap()
            .stake_pool
            .retirement
            .is_none(),
        "retirement field in explorer should be empty",
    )?;

    let mut david = controller.wallet("david")?;
    let mut spo_3 = stake_pool_3.owner().clone();

    let fragment_sender = FragmentSender::from(controller.settings());

    fragment_sender.send_transaction(&mut david, &spo_3, &leader_1, 100.into())?;

    fragment_sender.send_pool_retire(&mut spo_3, &stake_pool_3, &leader_1)?;

    std::thread::sleep(std::time::Duration::from_secs(70));

    let created_block_count = leader_3.logger().get_created_blocks_hashes().len();
    let start_time_no_block = std::time::SystemTime::now();

    // proof 1: explorer shows as retired
    let stake_pool_state_after = explorer.stake_pool(stake_pool_3.id().to_string(), 0)?;
    utils::assert(
        stake_pool_state_after
            .data
            .unwrap()
            .stake_pool
            .retirement
            .is_none(),
        "retirement field in explorer should not be empty",
    )?;

    // proof 2: minted block count not increased
    let created_blocks_count_after_retire = leader_3.logger().get_created_blocks_hashes().len();
    utils::assert(
        created_blocks_count_after_retire == created_block_count,
        "after retirement there are no new block minted",
    )?;

    //proof 3: no more minted blocks hashes in logs
    std::thread::sleep(std::time::Duration::from_secs(60));
    utils::assert(
        leader_3
            .logger()
            .get_created_blocks_hashes_after(start_time_no_block.into())
            .is_empty(),
        "leader 3 should not create any block after retirement",
    )?;

    leader_1.shutdown()?;
    leader_2.shutdown()?;
    leader_3.shutdown()?;
    leader_4.shutdown()?;
    controller.finalize();
    Ok(ScenarioResult::passed(name))
}
