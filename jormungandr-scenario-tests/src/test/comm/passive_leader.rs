use crate::{
    node::{LeadershipMode, PersistenceMode},
    test::utils::{self, SyncMeasurementInterval, SyncWaitParams},
    test::Result,
    Context, ScenarioResult,
};
use rand_chacha::ChaChaRng;

const LEADER: &str = "Leader";
const PASSIVE: &str = "Passive";

pub fn transaction_to_passive(mut context: Context<ChaChaRng>) -> Result<ScenarioResult> {
    let scenario_settings = prepare_scenario! {
        "L2001-transaction_propagation_from_passive",
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
                account "unassigned1" with   500_000_000,
                account "delegated1" with  2_000_000_000 delegates to LEADER,
            ],
        }
    };

    let mut controller = scenario_settings.build(context)?;

    controller.monitor_nodes();
    let leader =
        controller.spawn_node(LEADER, LeadershipMode::Leader, PersistenceMode::InMemory)?;
    leader.wait_for_bootstrap()?;
    let passive =
        controller.spawn_node(PASSIVE, LeadershipMode::Passive, PersistenceMode::InMemory)?;
    passive.wait_for_bootstrap()?;

    let mut wallet1 = controller.wallet("unassigned1")?;
    let mut wallet2 = controller.wallet("delegated1")?;

    utils::sending_transactions_to_node_sequentially(
        10,
        &mut controller,
        &mut wallet1,
        &mut wallet2,
        &passive,
    )?;

    utils::measure_and_log_sync_time(
        vec![&passive, &leader],
        SyncWaitParams::two_nodes().into(),
        "transaction_to_passive_sync",
        SyncMeasurementInterval::Standard,
    );

    passive.shutdown()?;
    leader.shutdown()?;
    controller.finalize();
    Ok(ScenarioResult::passed())
}

pub fn leader_restart(mut context: Context<ChaChaRng>) -> Result<ScenarioResult> {
    let scenario_settings = prepare_scenario! {
        "L2003-leader_is_restarted",
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
                account "unassigned1" with   500_000_000,
                account "delegated1" with  2_000_000_000 delegates to LEADER,
            ],
        }
    };

    let mut controller = scenario_settings.build(context)?;
    //monitor node disabled due to unsupported operation: restart node
    //controller.monitor_nodes();

    let leader =
        controller.spawn_node(LEADER, LeadershipMode::Leader, PersistenceMode::Persistent)?;
    leader.wait_for_bootstrap()?;

    let passive = controller.spawn_node(
        PASSIVE,
        LeadershipMode::Passive,
        PersistenceMode::Persistent,
    )?;

    passive.wait_for_bootstrap()?;

    let mut wallet1 = controller.wallet("unassigned1")?;
    let mut wallet2 = controller.wallet("delegated1")?;

    utils::sending_transactions_to_node_sequentially(
        10,
        &mut controller,
        &mut wallet1,
        &mut wallet2,
        &passive,
    )?;

    leader.shutdown()?;

    utils::sending_transactions_to_node_sequentially(
        10,
        &mut controller,
        &mut wallet1,
        &mut wallet2,
        &passive,
    )?;

    let leader =
        controller.spawn_node(LEADER, LeadershipMode::Leader, PersistenceMode::Persistent)?;

    utils::sending_transactions_to_node_sequentially(
        10,
        &mut controller,
        &mut wallet1,
        &mut wallet2,
        &passive,
    )?;

    utils::measure_and_log_sync_time(
        vec![&passive, &leader],
        SyncWaitParams::nodes_restart(2).into(),
        "leader_restart",
        SyncMeasurementInterval::Standard,
    );

    passive.shutdown()?;
    leader.shutdown()?;

    controller.finalize();
    Ok(ScenarioResult::passed())
}

pub fn passive_node_is_updated(mut context: Context<ChaChaRng>) -> Result<ScenarioResult> {
    let scenario_settings = prepare_scenario! {
        "L2004-passive_node_is_updated",
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
                account "unassigned1" with   500_000_000,
                account "delegated1" with  2_000_000_000 delegates to LEADER,
            ],
        }
    };

    let mut controller = scenario_settings.build(context)?;

    controller.monitor_nodes();

    let leader =
        controller.spawn_node(LEADER, LeadershipMode::Leader, PersistenceMode::InMemory)?;
    leader.wait_for_bootstrap()?;

    let passive =
        controller.spawn_node(PASSIVE, LeadershipMode::Passive, PersistenceMode::InMemory)?;
    passive.wait_for_bootstrap()?;

    let mut wallet1 = controller.wallet("unassigned1")?;
    let mut wallet2 = controller.wallet("delegated1")?;

    utils::sending_transactions_to_node_sequentially(
        40,
        &mut controller,
        &mut wallet1,
        &mut wallet2,
        &leader,
    )?;

    utils::measure_and_log_sync_time(
        vec![&passive, &leader],
        SyncWaitParams::nodes_restart(2).into(),
        "passive_node_is_updated_sync",
        SyncMeasurementInterval::Standard,
    );

    passive.shutdown()?;
    leader.shutdown()?;

    controller.finalize();
    Ok(ScenarioResult::passed())
}
