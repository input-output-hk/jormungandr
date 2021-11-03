use crate::{
    node::{LeadershipMode, PersistenceMode},
    test::utils::{self, MeasurementReportInterval, SyncWaitParams},
    test::Result,
    Context, ScenarioResult,
};
use jormungandr_lib::interfaces::Policy;
use std::time::Duration;

use jormungandr_testing_utils::testing::FragmentSenderSetup;
use rand_chacha::ChaChaRng;

const LEADER: &str = "Leader";
const PASSIVE: &str = "Passive";

use function_name::named;

#[named]
pub fn transaction_to_passive(mut context: Context<ChaChaRng>) -> Result<ScenarioResult> {
    let name = function_name!();
    let scenario_settings = prepare_scenario! {
        name,
        &mut context,
        topology [
            LEADER,
            PASSIVE -> LEADER,
        ]
        blockchain {
            consensus = GenesisPraos,
            number_of_slots_per_epoch = 60,
            slot_duration = 1,
            leaders = [ LEADER],
            initials = [
                "account" "unassigned1" with   500_000_000,
                "account" "delegated1" with  2_000_000_000 delegates to LEADER,
            ],
        }
    };

    let mut controller = scenario_settings.build(context)?;

    controller.monitor_nodes();
    let mut leader =
        controller.spawn_node(LEADER, LeadershipMode::Leader, PersistenceMode::InMemory)?;
    leader.wait_for_bootstrap()?;
    let mut passive =
        controller.spawn_node(PASSIVE, LeadershipMode::Passive, PersistenceMode::InMemory)?;
    passive.wait_for_bootstrap()?;

    let mut wallet1 = controller.wallet("unassigned1")?;
    let mut wallet2 = controller.wallet("delegated1")?;

    controller.fragment_sender().send_transactions_round_trip(
        10,
        &mut wallet1,
        &mut wallet2,
        &passive,
        1_000.into(),
    )?;

    utils::measure_and_log_sync_time(
        &[&passive, &leader],
        SyncWaitParams::two_nodes().into(),
        "transaction_to_passive_sync",
        MeasurementReportInterval::Standard,
    )?;

    passive.shutdown()?;
    leader.shutdown()?;
    controller.finalize();
    Ok(ScenarioResult::passed(name))
}

const LEADER_2: &str = "LEADER_2";

#[named]
pub fn leader_restart(mut context: Context<ChaChaRng>) -> Result<ScenarioResult> {
    let name = function_name!();
    let scenario_settings = prepare_scenario! {
        name,
        &mut context,
        topology [
            LEADER_2,
            LEADER -> LEADER_2,
            PASSIVE -> LEADER -> LEADER_2
        ]
        blockchain {
            consensus = GenesisPraos,
            number_of_slots_per_epoch = 60,
            slot_duration = 1,
            leaders = [ LEADER, LEADER_2],
            initials = [
                "account" "unassigned1" with   500_000_000,
                "account" "delegated1" with  2_000_000_000 delegates to LEADER,
                "account" "delegated1" with  2_000_000_000 delegates to LEADER_2,
            ],
        }
    };

    let mut controller = scenario_settings.build(context)?;
    //monitor node disabled due to unsupported operation: restart node
    //controller.monitor_nodes();

    let policy = Policy {
        quarantine_duration: Some(Duration::new(5, 0).into()),
        quarantine_whitelist: None,
    };

    let mut leader_2 = controller.spawn_node_custom(
        controller
            .new_spawn_params(LEADER_2)
            .policy(policy.clone())
            .leader(),
    )?;
    leader_2.wait_for_bootstrap()?;

    let mut leader =
        controller.spawn_node(LEADER, LeadershipMode::Leader, PersistenceMode::Persistent)?;
    leader.wait_for_bootstrap()?;

    let mut passive = controller.spawn_node_custom(
        controller
            .new_spawn_params(PASSIVE)
            .policy(policy)
            .passive(),
    )?;
    passive.wait_for_bootstrap()?;

    let mut wallet1 = controller.wallet("unassigned1")?;
    let mut wallet2 = controller.wallet("delegated1")?;

    controller
        .fragment_sender_with_setup(FragmentSenderSetup::resend_3_times())
        .send_transactions_round_trip(10, &mut wallet1, &mut wallet2, &passive, 1_000.into())?;

    leader.shutdown()?;

    controller
        .fragment_sender_with_setup(FragmentSenderSetup::resend_3_times())
        .send_transactions_with_iteration_delay(
            10,
            &mut wallet1,
            &mut wallet2,
            &passive,
            1_000.into(),
            Duration::from_secs(3),
        )?;

    let mut leader =
        controller.spawn_node(LEADER, LeadershipMode::Leader, PersistenceMode::Persistent)?;
    leader.wait_for_bootstrap()?;

    controller
        .fragment_sender_with_setup(FragmentSenderSetup::resend_3_times())
        .send_transactions_round_trip(10, &mut wallet1, &mut wallet2, &passive, 1_000.into())?;

    utils::measure_and_log_sync_time(
        &[&passive, &leader, &leader_2],
        SyncWaitParams::nodes_restart(2).into(),
        "leader_restart",
        MeasurementReportInterval::Standard,
    )?;

    passive.shutdown()?;
    leader.shutdown()?;
    leader_2.shutdown()?;

    controller.finalize();
    Ok(ScenarioResult::passed(name))
}

#[named]
pub fn passive_node_is_updated(mut context: Context<ChaChaRng>) -> Result<ScenarioResult> {
    let name = function_name!();
    let scenario_settings = prepare_scenario! {
        name,
        &mut context,
        topology [
            LEADER,
            PASSIVE -> LEADER,
        ]
        blockchain {
            consensus = GenesisPraos,
            number_of_slots_per_epoch = 60,
            slot_duration = 1,
            leaders = [ LEADER],
            initials = [
                "account" "unassigned1" with   500_000_000,
                "account" "delegated1" with  2_000_000_000 delegates to LEADER,
            ],
        }
    };

    let mut controller = scenario_settings.build(context)?;

    controller.monitor_nodes();

    let mut leader =
        controller.spawn_node(LEADER, LeadershipMode::Leader, PersistenceMode::InMemory)?;
    leader.wait_for_bootstrap()?;

    let mut passive =
        controller.spawn_node(PASSIVE, LeadershipMode::Passive, PersistenceMode::InMemory)?;
    passive.wait_for_bootstrap()?;

    let mut wallet1 = controller.wallet("unassigned1")?;
    let mut wallet2 = controller.wallet("delegated1")?;

    controller.fragment_sender().send_transactions_round_trip(
        40,
        &mut wallet1,
        &mut wallet2,
        &leader,
        1_000.into(),
    )?;

    utils::measure_and_log_sync_time(
        &[&passive, &leader],
        SyncWaitParams::nodes_restart(2).into(),
        "passive_node_is_updated_sync",
        MeasurementReportInterval::Standard,
    )?;

    passive.shutdown()?;
    leader.shutdown()?;

    controller.finalize();
    Ok(ScenarioResult::passed(name))
}
