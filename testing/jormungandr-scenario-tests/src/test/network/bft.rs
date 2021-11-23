use crate::{
    test::{
        utils::{self, MeasurementReportInterval, SyncWaitParams},
        Result,
    },
    Context, ScenarioResult,
};

use function_name::named;
use jormungandr_testing_utils::testing::network::{LeadershipMode, PersistenceMode};
use jormungandr_testing_utils::testing::FragmentSender;
use jormungandr_testing_utils::testing::FragmentVerifier;
use std::time::Duration;

const ALICE: &str = "Alice";
const BOB: &str = "Bob";
const LEADER_1: &str = "Leader1";
const LEADER_2: &str = "Leader2";
const LEADER_3: &str = "Leader3";
const LEADER_4: &str = "Leader4";
const LEADER_5: &str = "Leader5";
const PASSIVE: &str = "Passive";

#[named]
pub fn bft_cascade(context: Context) -> Result<ScenarioResult> {
    let name = function_name!();
    let scenario_settings = prepare_scenario! {
        name,
        &mut context,
        topology [
            LEADER_1,
            LEADER_2 -> LEADER_1,
            LEADER_3 -> LEADER_2 -> LEADER_1,
            LEADER_4 -> LEADER_3 -> LEADER_2,
            LEADER_5 -> LEADER_4 -> LEADER_3,
        ]
        blockchain {
            consensus = Bft,
            number_of_slots_per_epoch = 60,
            slot_duration = 1,
            leaders = [ LEADER_1, LEADER_2, LEADER_3, LEADER_4,LEADER_5 ],
            initials = [
                "account" ALICE with   500_000_000,
                "account" BOB with  500_000_000,
            ],
        }
    };

    let mut controller = scenario_settings.build(context)?;

    controller.monitor_nodes();

    let mut leader1 =
        controller.spawn_node(LEADER_1, LeadershipMode::Leader, PersistenceMode::InMemory)?;
    leader1.wait_for_bootstrap()?;

    let mut leader2 =
        controller.spawn_node(LEADER_2, LeadershipMode::Leader, PersistenceMode::InMemory)?;
    leader2.wait_for_bootstrap()?;

    let mut leader3 =
        controller.spawn_node(LEADER_3, LeadershipMode::Leader, PersistenceMode::InMemory)?;
    leader3.wait_for_bootstrap()?;

    let mut leader4 =
        controller.spawn_node(LEADER_4, LeadershipMode::Leader, PersistenceMode::InMemory)?;
    leader4.wait_for_bootstrap()?;

    let mut leader5 =
        controller.spawn_node(LEADER_5, LeadershipMode::Leader, PersistenceMode::InMemory)?;
    leader5.wait_for_bootstrap()?;

    let leaders = [&leader1, &leader2, &leader3, &leader4, &leader5];

    utils::measure_and_log_sync_time(
        &leaders,
        SyncWaitParams::network_size(5, 3).into(),
        "bft cascade sync",
        MeasurementReportInterval::Standard,
    )?;

    // let mut leader5 =
    //     controller.spawn_node_custom(controller.new_spawn_params(PASSIVE).passive().explorer(Explorer{enabled: true}))?;
    // passive.wait_for_bootstrap()?;

    let mut alice = controller.wallet(ALICE)?;
    let mut bob = controller.wallet(BOB)?;

    std::thread::sleep(std::time::Duration::from_secs(60));

    FragmentSender::from(controller.settings()).send_transactions_round_trip(
        40,
        &mut alice,
        &mut bob,
        &leader5,
        1_000.into(),
    )?;

    utils::measure_and_log_sync_time(
        &leaders,
        SyncWaitParams::network_size(5, 3).into(),
        "bft cascade sync",
        MeasurementReportInterval::Standard,
    )?;

    leader5.shutdown()?;
    leader4.shutdown()?;
    leader3.shutdown()?;
    leader2.shutdown()?;
    leader1.shutdown()?;
    Ok(ScenarioResult::passed(name))
}

#[named]
pub fn bft_passive_propagation(context: Context) -> Result<ScenarioResult> {
    let name = function_name!();
    let scenario_settings = prepare_scenario! {
        "bft_passive_propagation",
        &mut context,
        topology [
            LEADER_3,
            LEADER_1 -> LEADER_3 -> PASSIVE,
            LEADER_2 -> LEADER_1,
            PASSIVE -> LEADER_2 -> LEADER_3,
        ]
        blockchain {
            consensus = Bft,
            number_of_slots_per_epoch = 60,
            slot_duration = 1,
            leaders = [ LEADER_1, LEADER_2, LEADER_3 ],
            initials = [
                "account" ALICE with  2_000_000_000,
                "account" BOB with  2_000_000_000,
            ],
        }
    };

    let mut controller = scenario_settings.build(context)?;

    let mut leader1 =
        controller.spawn_node(LEADER_1, LeadershipMode::Leader, PersistenceMode::InMemory)?;

    let mut leader2 =
        controller.spawn_node(LEADER_2, LeadershipMode::Leader, PersistenceMode::InMemory)?;

    let mut leader3 =
        controller.spawn_node(LEADER_3, LeadershipMode::Leader, PersistenceMode::InMemory)?;

    let mut passive =
        controller.spawn_node_custom(controller.new_spawn_params(PASSIVE).passive().in_memory())?;

    controller.monitor_nodes();

    leader1.wait_for_bootstrap()?;
    leader2.wait_for_bootstrap()?;
    leader3.wait_for_bootstrap()?;
    passive.wait_for_bootstrap()?;

    let nodes = [&leader1, &leader2, &leader3, &passive];

    utils::measure_and_log_sync_time(
        &nodes,
        SyncWaitParams::network_size(4, 3).into(),
        "bft passive propagation sync",
        MeasurementReportInterval::Standard,
    )?;

    let mut alice_wallet = controller.wallet(ALICE)?;
    let bob_wallet = controller.wallet(BOB)?;

    let mem_pool_check = FragmentSender::from(controller.settings()).send_transaction(
        &mut alice_wallet,
        &bob_wallet,
        &leader2,
        1_000.into(),
    )?;

    FragmentVerifier::wait_and_verify_is_in_block(
        Duration::new(2, 0),
        mem_pool_check.clone(),
        &leader1,
    )?;

    FragmentVerifier::wait_and_verify_is_in_block(
        Duration::new(2, 0),
        mem_pool_check.clone(),
        &leader2,
    )?;

    FragmentVerifier::wait_and_verify_is_in_block(
        Duration::new(2, 0),
        mem_pool_check.clone(),
        &leader3,
    )?;

    FragmentVerifier::wait_and_verify_is_in_block(Duration::new(2, 0), mem_pool_check, &passive)?;

    leader1.shutdown()?;
    leader2.shutdown()?;
    leader3.shutdown()?;
    passive.shutdown()?;

    Ok(ScenarioResult::passed(name))
}
