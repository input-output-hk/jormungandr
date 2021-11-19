use crate::{
    test::{
        utils::{self, MeasurementReportInterval, SyncWaitParams},
        Result,
    },
    Context, ScenarioResult,
};
use jormungandr_testing_utils::testing::network::{LeadershipMode, PersistenceMode};
use jormungandr_testing_utils::testing::FragmentSender;
use jormungandr_testing_utils::testing::FragmentSenderSetup;

const LEADER_1: &str = "Leader1";
const LEADER_2: &str = "Leader2";
const LEADER_3: &str = "Leader3";
const LEADER_4: &str = "Leader4";
const LEADER_5: &str = "Leader5";
const LEADER_6: &str = "Leader6";
const LEADER_7: &str = "Leader7";

const CORE_NODE: &str = "Core";
const RELAY_NODE_1: &str = "Relay1";
const RELAY_NODE_2: &str = "Relay2";

use function_name::named;

#[named]
pub fn fully_connected(context: Context) -> Result<ScenarioResult> {
    let name = function_name!();
    let scenario_settings = prepare_scenario! {
        "T3001_Fully-Connected",
        &mut context,
        topology [
            LEADER_3,
            LEADER_1 -> LEADER_3,LEADER_4,
            LEADER_2 -> LEADER_1,
            LEADER_4 -> LEADER_2,LEADER_3,
        ]
        blockchain {
            consensus = GenesisPraos,
            number_of_slots_per_epoch = 60,
            slot_duration = 1,
            leaders = [ LEADER_1 ],
            initials = [
                "account" "unassigned1" with   500_000_000,
                "account" "delegated1" with  2_000_000_000 delegates to LEADER_2,
            ],
        }
    };

    let mut controller = scenario_settings.build(context)?;

    controller.monitor_nodes();
    let mut leader4 =
        controller.spawn_node(LEADER_4, LeadershipMode::Leader, PersistenceMode::InMemory)?;
    let mut leader3 =
        controller.spawn_node(LEADER_3, LeadershipMode::Leader, PersistenceMode::InMemory)?;
    let mut leader2 =
        controller.spawn_node(LEADER_2, LeadershipMode::Leader, PersistenceMode::InMemory)?;
    let mut leader1 =
        controller.spawn_node(LEADER_1, LeadershipMode::Leader, PersistenceMode::InMemory)?;

    leader4.wait_for_bootstrap()?;
    leader3.wait_for_bootstrap()?;
    leader2.wait_for_bootstrap()?;
    leader1.wait_for_bootstrap()?;

    let mut wallet1 = controller.wallet("unassigned1")?;
    let mut wallet2 = controller.wallet("delegated1")?;

    let fragment_sender = FragmentSender::from(controller.settings());

    fragment_sender.send_transactions_round_trip(
        10,
        &mut wallet1,
        &mut wallet2,
        &leader1,
        1_000.into(),
    )?;

    let leaders = [&leader1, &leader2, &leader3, &leader4];

    utils::measure_and_log_sync_time(
        &leaders,
        SyncWaitParams::network_size(4, 2).into(),
        "fully_connected_sync",
        MeasurementReportInterval::Standard,
    )?;

    utils::measure_single_transaction_propagation_speed(
        &mut controller,
        &mut wallet1,
        &wallet2,
        &leaders,
        SyncWaitParams::network_size(4, 2).into(),
        "fully_connected_single_transaction_propagation",
        MeasurementReportInterval::Standard,
    )?;

    leader4.shutdown()?;
    leader3.shutdown()?;
    leader2.shutdown()?;
    leader1.shutdown()?;

    controller.finalize();
    Ok(ScenarioResult::passed(name))
}

#[named]
pub fn star(context: Context) -> Result<ScenarioResult> {
    let name = function_name!();
    let scenario_settings = prepare_scenario! {
        name,
        &mut context,
        topology [
            LEADER_5,
            LEADER_1 -> LEADER_5,
            LEADER_2 -> LEADER_5,
            LEADER_3 -> LEADER_5,
            LEADER_4 -> LEADER_5,
        ]
        blockchain {
            consensus = GenesisPraos,
            number_of_slots_per_epoch = 60,
            slot_duration = 1,
            leaders = [ LEADER_1 ],
            initials = [
                "account" "unassigned1" with   500_000_000,
                "account" "delegated1" with  2_000_000_000 delegates to LEADER_5,
            ],
        }
    };

    let mut controller = scenario_settings.build(context)?;

    controller.monitor_nodes();
    let mut leader5 =
        controller.spawn_node(LEADER_5, LeadershipMode::Leader, PersistenceMode::InMemory)?;
    let mut leader4 =
        controller.spawn_node(LEADER_4, LeadershipMode::Leader, PersistenceMode::InMemory)?;
    let mut leader3 =
        controller.spawn_node(LEADER_3, LeadershipMode::Leader, PersistenceMode::InMemory)?;
    let mut leader2 =
        controller.spawn_node(LEADER_2, LeadershipMode::Leader, PersistenceMode::InMemory)?;
    let mut leader1 =
        controller.spawn_node(LEADER_1, LeadershipMode::Leader, PersistenceMode::InMemory)?;

    leader5.wait_for_bootstrap()?;
    leader4.wait_for_bootstrap()?;
    leader3.wait_for_bootstrap()?;
    leader2.wait_for_bootstrap()?;
    leader1.wait_for_bootstrap()?;

    let mut wallet1 = controller.wallet("unassigned1")?;
    let mut wallet2 = controller.wallet("delegated1")?;

    FragmentSender::from(controller.settings()).send_transactions_round_trip(
        40,
        &mut wallet1,
        &mut wallet2,
        &leader1,
        1_000.into(),
    )?;

    let leaders = [&leader1, &leader2, &leader3, &leader4, &leader5];
    utils::measure_and_log_sync_time(
        &leaders,
        SyncWaitParams::network_size(5, 3).into(),
        "star_sync",
        MeasurementReportInterval::Standard,
    )?;

    utils::measure_single_transaction_propagation_speed(
        &mut controller,
        &mut wallet1,
        &wallet2,
        &leaders,
        SyncWaitParams::network_size(5, 3).into(),
        "star_single_transaction_propagation",
        MeasurementReportInterval::Standard,
    )?;

    leader5.shutdown()?;
    leader4.shutdown()?;
    leader3.shutdown()?;
    leader2.shutdown()?;
    leader1.shutdown()?;

    controller.finalize();
    Ok(ScenarioResult::passed(name))
}

#[named]
pub fn mesh(context: Context) -> Result<ScenarioResult> {
    let name = function_name!();
    let scenario_settings = prepare_scenario! {
        name,
        &mut context,
        topology [
            LEADER_4,
            LEADER_1 -> LEADER_4,
            LEADER_2 -> LEADER_1 -> LEADER_4,
            LEADER_3 -> LEADER_1 -> LEADER_2,
            LEADER_5 -> LEADER_2 -> LEADER_1,
        ]
        blockchain {
            consensus = GenesisPraos,
            number_of_slots_per_epoch = 60,
            slot_duration = 1,
            leaders = [ LEADER_1 ],
            initials = [
                "account" "unassigned1" with   500_000_000,
                "account" "delegated1" with  2_000_000_000 delegates to LEADER_3,
            ],
        }
    };

    let mut controller = scenario_settings.build(context)?;

    let mut leader4 =
        controller.spawn_node(LEADER_4, LeadershipMode::Leader, PersistenceMode::InMemory)?;
    leader4.wait_for_bootstrap()?;

    let mut leader1 =
        controller.spawn_node(LEADER_1, LeadershipMode::Leader, PersistenceMode::InMemory)?;
    leader1.wait_for_bootstrap()?;

    let mut leader2 =
        controller.spawn_node(LEADER_2, LeadershipMode::Leader, PersistenceMode::InMemory)?;
    leader2.wait_for_bootstrap()?;

    let mut leader3 =
        controller.spawn_node(LEADER_3, LeadershipMode::Leader, PersistenceMode::InMemory)?;
    leader3.wait_for_bootstrap()?;

    let mut leader5 =
        controller.spawn_node(LEADER_5, LeadershipMode::Leader, PersistenceMode::InMemory)?;
    leader5.wait_for_bootstrap()?;

    controller.monitor_nodes();

    let mut wallet1 = controller.wallet("unassigned1")?;
    let mut wallet2 = controller.wallet("delegated1")?;

    FragmentSender::from(controller.settings()).send_transactions_round_trip(
        4,
        &mut wallet1,
        &mut wallet2,
        &leader1,
        1_000.into(),
    )?;

    let leaders = [&leader1, &leader2, &leader3, &leader4, &leader5];

    utils::measure_and_log_sync_time(
        &leaders,
        SyncWaitParams::network_size(5, 3).into(),
        "mesh_sync",
        MeasurementReportInterval::Standard,
    )?;

    utils::measure_single_transaction_propagation_speed(
        &mut controller,
        &mut wallet1,
        &wallet2,
        &leaders,
        SyncWaitParams::network_size(5, 3).into(),
        "mesh_single_transaction_propagation",
        MeasurementReportInterval::Standard,
    )?;

    leader5.shutdown()?;
    leader4.shutdown()?;
    leader3.shutdown()?;
    leader2.shutdown()?;
    leader1.shutdown()?;

    controller.finalize();

    Ok(ScenarioResult::passed(name))
}

#[named]
pub fn point_to_point(context: Context) -> Result<ScenarioResult> {
    let name = function_name!();
    let scenario_settings = prepare_scenario! {
        name,
        &mut context,
        topology [
            LEADER_4,
            LEADER_3 -> LEADER_4,
            LEADER_2 -> LEADER_3,
            LEADER_1 -> LEADER_2,
        ]
        blockchain {
            consensus = GenesisPraos,
            number_of_slots_per_epoch = 60,
            slot_duration = 1,
            leaders = [ LEADER_1 ],
            initials = [
                "account" "unassigned1" with   500_000_000,
                "account" "delegated1" with  2_000_000_000 delegates to LEADER_1,
            ],
        }
    };

    let mut controller = scenario_settings.build(context)?;

    controller.monitor_nodes();
    let mut leader4 =
        controller.spawn_node(LEADER_4, LeadershipMode::Leader, PersistenceMode::InMemory)?;
    let mut leader3 =
        controller.spawn_node(LEADER_3, LeadershipMode::Leader, PersistenceMode::InMemory)?;
    let mut leader2 =
        controller.spawn_node(LEADER_2, LeadershipMode::Leader, PersistenceMode::InMemory)?;
    let mut leader1 =
        controller.spawn_node(LEADER_1, LeadershipMode::Leader, PersistenceMode::InMemory)?;

    leader4.wait_for_bootstrap()?;
    leader3.wait_for_bootstrap()?;
    leader2.wait_for_bootstrap()?;
    leader1.wait_for_bootstrap()?;

    let mut wallet1 = controller.wallet("unassigned1")?;
    let mut wallet2 = controller.wallet("delegated1")?;

    FragmentSender::from(controller.settings()).send_transactions_round_trip(
        40,
        &mut wallet1,
        &mut wallet2,
        &leader1,
        1_000.into(),
    )?;

    let leaders = [&leader1, &leader2, &leader3, &leader4];

    utils::measure_and_log_sync_time(
        &leaders,
        SyncWaitParams::network_size(4, 4).into(),
        "point_to_point_sync",
        MeasurementReportInterval::Standard,
    )?;

    utils::measure_single_transaction_propagation_speed(
        &mut controller,
        &mut wallet1,
        &wallet2,
        &leaders,
        SyncWaitParams::network_size(4, 4).into(),
        "point_to_point_single_transaction_propagation",
        MeasurementReportInterval::Standard,
    )?;

    leader4.shutdown()?;
    leader3.shutdown()?;
    leader2.shutdown()?;
    leader1.shutdown()?;

    controller.finalize();
    Ok(ScenarioResult::passed(name))
}

#[named]
pub fn point_to_point_on_file_storage(context: Context) -> Result<ScenarioResult> {
    let name = function_name!();
    let scenario_settings = prepare_scenario! {
        name,
        &mut context,
        topology [
            LEADER_4,
            LEADER_3 -> LEADER_4,
            LEADER_2 -> LEADER_3,
            LEADER_1 -> LEADER_2,
        ]
        blockchain {
            consensus = GenesisPraos,
            number_of_slots_per_epoch = 60,
            slot_duration = 1,
            leaders = [ LEADER_1 ],
            initials = [
                "account" "unassigned1" with   500_000_000,
                "account" "delegated1" with  2_000_000_000 delegates to LEADER_1,
            ],
        }
    };

    let mut controller = scenario_settings.build(context)?;

    controller.monitor_nodes();
    let mut leader4 = controller.spawn_node(
        LEADER_4,
        LeadershipMode::Leader,
        PersistenceMode::Persistent,
    )?;
    let mut leader3 = controller.spawn_node(
        LEADER_3,
        LeadershipMode::Leader,
        PersistenceMode::Persistent,
    )?;
    let mut leader2 = controller.spawn_node(
        LEADER_2,
        LeadershipMode::Leader,
        PersistenceMode::Persistent,
    )?;
    let mut leader1 = controller.spawn_node(
        LEADER_1,
        LeadershipMode::Leader,
        PersistenceMode::Persistent,
    )?;

    leader4.wait_for_bootstrap()?;
    leader3.wait_for_bootstrap()?;
    leader2.wait_for_bootstrap()?;
    leader1.wait_for_bootstrap()?;

    let mut wallet1 = controller.wallet("unassigned1")?;
    let mut wallet2 = controller.wallet("delegated1")?;

    FragmentSender::from(controller.settings()).send_transactions_round_trip(
        40,
        &mut wallet1,
        &mut wallet2,
        &leader1,
        1_000.into(),
    )?;

    let leaders = [&leader1, &leader2, &leader3, &leader4];

    utils::measure_and_log_sync_time(
        &leaders,
        SyncWaitParams::network_size(4, 4).into(),
        "point_to_point_on_file_storage_sync",
        MeasurementReportInterval::Standard,
    )?;

    utils::measure_single_transaction_propagation_speed(
        &mut controller,
        &mut wallet1,
        &wallet2,
        &leaders,
        SyncWaitParams::network_size(4, 4).into(),
        "point_to_point_on_file_storage_single_transaction_propagation",
        MeasurementReportInterval::Standard,
    )?;

    leader4.shutdown()?;
    leader3.shutdown()?;
    leader2.shutdown()?;
    leader1.shutdown()?;

    controller.finalize();
    Ok(ScenarioResult::passed(name))
}

#[named]
pub fn tree(context: Context) -> Result<ScenarioResult> {
    let name = function_name!();
    let scenario_settings = prepare_scenario! {
        name,
        &mut context,
        topology [
            LEADER_1,
            LEADER_2 -> LEADER_1,
            LEADER_3 -> LEADER_1,
            LEADER_4 -> LEADER_2,
            LEADER_5 -> LEADER_2,
            LEADER_6 -> LEADER_3,
            LEADER_7 -> LEADER_3
        ]
        blockchain {
            consensus = GenesisPraos,
            number_of_slots_per_epoch = 60,
            slot_duration = 1,
            leaders = [ LEADER_1 ],
            initials = [
                "account" "unassigned1" with   500_000_000,
                "account" "delegated1" with  2_000_000_000 delegates to LEADER_7,
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
    let mut leader4 =
        controller.spawn_node(LEADER_4, LeadershipMode::Leader, PersistenceMode::InMemory)?;
    let mut leader5 =
        controller.spawn_node(LEADER_5, LeadershipMode::Leader, PersistenceMode::InMemory)?;
    let mut leader6 =
        controller.spawn_node(LEADER_6, LeadershipMode::Leader, PersistenceMode::InMemory)?;
    let mut leader7 =
        controller.spawn_node(LEADER_7, LeadershipMode::Leader, PersistenceMode::InMemory)?;

    controller.monitor_nodes();
    leader7.wait_for_bootstrap()?;
    leader6.wait_for_bootstrap()?;
    leader5.wait_for_bootstrap()?;
    leader4.wait_for_bootstrap()?;
    leader3.wait_for_bootstrap()?;
    leader2.wait_for_bootstrap()?;
    leader1.wait_for_bootstrap()?;

    let mut wallet1 = controller.wallet("unassigned1")?;
    let mut wallet2 = controller.wallet("delegated1")?;

    FragmentSender::from(controller.settings()).send_transactions_round_trip(
        40,
        &mut wallet1,
        &mut wallet2,
        &leader1,
        1_000.into(),
    )?;

    let leaders = [
        &leader1, &leader2, &leader3, &leader4, &leader5, &leader6, &leader7,
    ];

    utils::measure_and_log_sync_time(
        &leaders,
        SyncWaitParams::network_size(7, 5).into(),
        "tree_sync",
        MeasurementReportInterval::Standard,
    )?;

    utils::measure_single_transaction_propagation_speed(
        &mut controller,
        &mut wallet1,
        &wallet2,
        &leaders,
        SyncWaitParams::network_size(7, 5).into(),
        "tree_single_transaction_propagation",
        MeasurementReportInterval::Standard,
    )?;

    leader7.shutdown()?;
    leader6.shutdown()?;
    leader5.shutdown()?;
    leader4.shutdown()?;
    leader3.shutdown()?;
    leader2.shutdown()?;
    leader1.shutdown()?;

    controller.finalize();
    Ok(ScenarioResult::passed(name))
}

#[named]
pub fn relay(context: Context) -> Result<ScenarioResult> {
    let name = function_name!();
    let scenario_settings = prepare_scenario! {
        name,
        &mut context,
        topology [
            CORE_NODE,
            RELAY_NODE_1 -> CORE_NODE,
            RELAY_NODE_2 -> CORE_NODE,
            LEADER_1 -> RELAY_NODE_1,
            LEADER_2 -> RELAY_NODE_1,
            LEADER_3 -> RELAY_NODE_1,
            LEADER_4 -> RELAY_NODE_2,
            LEADER_5 -> RELAY_NODE_2,
            LEADER_6 -> RELAY_NODE_2,
            LEADER_7 -> RELAY_NODE_2
        ]
        blockchain {
            consensus = GenesisPraos,
            number_of_slots_per_epoch = 60,
            slot_duration = 1,
            leaders = [ LEADER_1 ],
            initials = [
                "account" "delegated1" with  1_000_000_000 delegates to LEADER_1,
                "account" "delegated2" with  1_000_000_000 delegates to LEADER_2,
                "account" "delegated3" with  1_000_000_000 delegates to LEADER_3,
                "account" "delegated4" with  1_000_000_000 delegates to LEADER_4,
                "account" "delegated5" with  1_000_000_000 delegates to LEADER_5,
                "account" "delegated6" with  1_000_000_000 delegates to LEADER_6,
                "account" "delegated7" with  1_000_000_000 delegates to LEADER_7,
            ],
        }
    };

    let mut controller = scenario_settings.build(context)?;

    let mut core =
        controller.spawn_node(CORE_NODE, LeadershipMode::Leader, PersistenceMode::InMemory)?;

    controller.monitor_nodes();
    core.wait_for_bootstrap()?;

    let mut relay1 = controller.spawn_node(
        RELAY_NODE_1,
        LeadershipMode::Passive,
        PersistenceMode::InMemory,
    )?;
    let mut relay2 = controller.spawn_node(
        RELAY_NODE_2,
        LeadershipMode::Passive,
        PersistenceMode::InMemory,
    )?;

    relay2.wait_for_bootstrap()?;
    relay1.wait_for_bootstrap()?;

    let mut leader1 =
        controller.spawn_node(LEADER_1, LeadershipMode::Leader, PersistenceMode::InMemory)?;
    let mut leader2 =
        controller.spawn_node(LEADER_2, LeadershipMode::Leader, PersistenceMode::InMemory)?;
    let mut leader3 =
        controller.spawn_node(LEADER_3, LeadershipMode::Leader, PersistenceMode::InMemory)?;
    let mut leader4 =
        controller.spawn_node(LEADER_4, LeadershipMode::Leader, PersistenceMode::InMemory)?;
    let mut leader5 =
        controller.spawn_node(LEADER_5, LeadershipMode::Leader, PersistenceMode::InMemory)?;
    let mut leader6 =
        controller.spawn_node(LEADER_6, LeadershipMode::Leader, PersistenceMode::InMemory)?;
    let mut leader7 =
        controller.spawn_node(LEADER_7, LeadershipMode::Leader, PersistenceMode::InMemory)?;

    leader1.wait_for_bootstrap()?;
    leader2.wait_for_bootstrap()?;
    leader3.wait_for_bootstrap()?;
    leader4.wait_for_bootstrap()?;
    leader5.wait_for_bootstrap()?;
    leader6.wait_for_bootstrap()?;
    leader7.wait_for_bootstrap()?;

    let mut wallet1 = controller.wallet("delegated1")?;
    let mut wallet2 = controller.wallet("delegated2")?;

    let setup = FragmentSenderSetup::resend_3_times_and_sync_with(vec![&core, &relay1, &relay2]);

    FragmentSender::from(controller.settings())
        .clone_with_setup(setup)
        .send_transactions_round_trip(40, &mut wallet1, &mut wallet2, &leader1, 1_000.into())?;

    let leaders = [
        &leader1, &leader2, &leader3, &leader4, &leader5, &leader6, &leader7, &relay1, &relay2,
        &core,
    ];

    utils::measure_and_log_sync_time(
        &leaders,
        SyncWaitParams::network_size(10, 3).into(),
        "relay_sync",
        MeasurementReportInterval::Standard,
    )?;

    utils::measure_single_transaction_propagation_speed(
        &mut controller,
        &mut wallet1,
        &wallet2,
        &leaders,
        SyncWaitParams::network_size(10, 3).into(),
        "relay_single_transaction_propagation",
        MeasurementReportInterval::Standard,
    )?;

    leader7.shutdown()?;
    leader6.shutdown()?;
    leader5.shutdown()?;
    leader4.shutdown()?;
    leader3.shutdown()?;
    leader2.shutdown()?;
    leader1.shutdown()?;

    relay1.shutdown()?;
    relay2.shutdown()?;

    core.shutdown()?;

    controller.finalize();
    Ok(ScenarioResult::passed(name))
}
