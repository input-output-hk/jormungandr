use crate::{
    node::{LeadershipMode, PersistenceMode},
    scenario::repository::ScenarioResult,
    test::{
        non_functional::*,
        utils::{self, MeasurementReportInterval, SyncWaitParams},
        Result,
    },
    Context,
};

use jormungandr_testing_utils::{testing::network_builder::FaketimeConfig, wallet::Wallet};

use function_name::named;
use rand_chacha::ChaChaRng;

#[named]
pub fn passive_leader_disruption_no_overlap(
    mut context: Context<ChaChaRng>,
) -> Result<ScenarioResult> {
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
            leaders = [ LEADER_1 ],
            initials = [
                "account" "unassigned1" with   500_000_000,
                "account" "delegated1" with  2_000_000_000 delegates to LEADER,
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

    // 1. both nodes are up
    utils::wait(5);

    // 2. Only passive is down
    leader.shutdown()?;

    // 3. No node is up
    passive.shutdown()?;

    // 4. Only leader is up
    let leader =
        controller.spawn_node(LEADER, LeadershipMode::Leader, PersistenceMode::Persistent)?;
    leader.wait_for_bootstrap()?;
    utils::wait(5);

    // 5. No node is up
    leader.shutdown()?;

    //6. Both nodes are up
    let leader =
        controller.spawn_node(LEADER, LeadershipMode::Leader, PersistenceMode::Persistent)?;
    let passive = controller.spawn_node(
        PASSIVE,
        LeadershipMode::Passive,
        PersistenceMode::Persistent,
    )?;

    leader.wait_for_bootstrap()?;
    passive.wait_for_bootstrap()?;

    utils::measure_and_log_sync_time(
        &[&leader, &passive],
        SyncWaitParams::nodes_restart(5).into(),
        "passive_leader_disruption_no_overlap",
        MeasurementReportInterval::Standard,
    )?;

    leader.shutdown()?;
    passive.shutdown()?;
    controller.finalize();
    Ok(ScenarioResult::passed(name))
}

#[named]
pub fn passive_leader_disruption_overlap(
    mut context: Context<ChaChaRng>,
) -> Result<ScenarioResult> {
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
            leaders = [ LEADER_1 ],
            initials = [
                "account" "unassigned1" with   500_000_000,
                "account" "delegated1" with  2_000_000_000 delegates to LEADER,
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

    // 1. both nodes are up
    utils::wait(5);

    // 2. Only leader is up
    passive.shutdown()?;

    // Wait a bit so that the leader can indeed notice that passive is offline
    utils::wait(15);

    // 3. Both nodes are up
    let passive = controller.spawn_node(
        PASSIVE,
        LeadershipMode::Passive,
        PersistenceMode::Persistent,
    )?;
    passive.wait_for_bootstrap()?;

    utils::measure_and_log_sync_time(
        &[&leader, &passive],
        SyncWaitParams::nodes_restart(5).into(),
        "passive_leader_disruption_overlap",
        MeasurementReportInterval::Standard,
    )?;

    leader.shutdown()?;
    passive.shutdown()?;
    controller.finalize();
    Ok(ScenarioResult::passed(name))
}

#[named]
pub fn bft_forks(mut context: Context<ChaChaRng>) -> Result<ScenarioResult> {
    let name = function_name!();
    let scenario_settings = prepare_scenario! {
        name,
        &mut context,
        topology [
            LEADER_1,
            LEADER_2 -> LEADER_1,
            LEADER_3 -> LEADER_1,
        ]
        blockchain {
            consensus = Bft,
            number_of_slots_per_epoch = 60,
            slot_duration = 5,
            leaders = [ LEADER_1, LEADER_2, LEADER_3 ],
            initials = [
                "account" "alice" with   100_000_000,
                "account" "bob" with   100_000_000,
            ],
        }
    };

    let mut controller = scenario_settings.build(context)?;

    let leader_1 = controller.spawn_node(
        LEADER_1,
        LeadershipMode::Leader,
        PersistenceMode::Persistent,
    )?;
    leader_1.wait_for_bootstrap()?;
    let leader_2 = controller.spawn_node(
        LEADER_2,
        LeadershipMode::Leader,
        PersistenceMode::Persistent,
    )?;
    leader_2.wait_for_bootstrap()?;
    let leader_3 = controller.spawn_node_custom(
        controller
            .new_spawn_params(LEADER_3)
            .leadership_mode(LeadershipMode::Leader)
            .persistence_mode(PersistenceMode::Persistent)
            .faketime(FaketimeConfig {
                offset: -2,
                drift: 0.0,
            }),
    )?;
    leader_3.wait_for_bootstrap()?;

    let mut alice = controller.wallet("alice")?;
    let bob = controller.wallet("bob")?;

    for i in 0..3 {
        // Sooner or later this will fail because a transaction will settle
        // in the fork and the spending counter will not be correct anymore
        let mut alice_clone = alice.clone();
        //println!("{:?} | {:?}", alice, alice_clone);
        controller.fragment_sender().send_transaction(
            &mut alice_clone,
            &bob,
            &leader_1,
            // so the transaction is not the same
            (1_000_000 + i).into(),
        )?;
        //alice = alice_clone;
        let state = leader_1.rest().account_state(&alice).unwrap();
        if let Wallet::Account(account) = &alice {
            let counter: u32 = account.internal_counter().into();
            if counter < state.counter() {
                alice.confirm_transaction();
            }
        }
        // Spans at least one slot for every leader
        std::thread::sleep(std::time::Duration::from_secs(5));
    }

    let account_value: u64 = leader_1
        .rest()
        .account_state(&alice)
        .unwrap()
        .value()
        .clone()
        .into();
    assert!(
        account_value < 100_000_000 - 1_000_000 * 3,
        "found {}",
        account_value
    );

    leader_1.shutdown()?;
    leader_2.shutdown()?;
    leader_3.shutdown()?;
    controller.finalize();
    Ok(ScenarioResult::passed(name))
}

#[named]
pub fn leader_leader_disruption_overlap(mut context: Context<ChaChaRng>) -> Result<ScenarioResult> {
    let name = function_name!();
    let scenario_settings = prepare_scenario! {
        name,
        &mut context,
        topology [
            LEADER_2,
            LEADER_1 -> LEADER_2,
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

    //monitor node disabled due to unsupported operation: restart node
    //controller.monitor_nodes();

    let leader2 = controller.spawn_node(
        LEADER_2,
        LeadershipMode::Leader,
        PersistenceMode::Persistent,
    )?;
    leader2.wait_for_bootstrap()?;

    // 1. second node is up
    utils::wait(5);

    // 2. Both nodes are up
    let leader1 = controller.spawn_node(
        LEADER_1,
        LeadershipMode::Leader,
        PersistenceMode::Persistent,
    )?;
    leader1.wait_for_bootstrap()?;
    utils::wait(5);

    // 3. second node is down
    leader2.shutdown()?;
    utils::wait(15);

    // 4. both nodes are up
    let leader2 = controller.spawn_node(
        LEADER_2,
        LeadershipMode::Leader,
        PersistenceMode::Persistent,
    )?;
    leader2.wait_for_bootstrap()?;

    utils::measure_and_log_sync_time(
        &[&leader1, &leader2],
        SyncWaitParams::nodes_restart(5).into(),
        "leader_leader_disruption_overlap",
        MeasurementReportInterval::Standard,
    )?;

    leader1.shutdown()?;
    leader2.shutdown()?;
    controller.finalize();
    Ok(ScenarioResult::passed(name))
}

#[named]
pub fn leader_leader_disruption_no_overlap(
    mut context: Context<ChaChaRng>,
) -> Result<ScenarioResult> {
    let name = function_name!();
    let scenario_settings = prepare_scenario! {
        name,
        &mut context,
        topology [
            LEADER_2,
            LEADER_1 -> LEADER_2,
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

    //monitor node disabled due to unsupported operation: restart node
    //controller.monitor_nodes();

    let leader2 = controller.spawn_node(
        LEADER_2,
        LeadershipMode::Leader,
        PersistenceMode::Persistent,
    )?;

    leader2.wait_for_bootstrap()?;

    let leader1 = controller.spawn_node(
        LEADER_1,
        LeadershipMode::Leader,
        PersistenceMode::Persistent,
    )?;
    leader1.wait_for_bootstrap()?;

    // 1. Both nodes are up
    utils::wait(5);

    // 2. Only node 2 is up
    leader1.shutdown()?;

    // 3. No nodes are up
    leader2.shutdown()?;

    // 4.- 5. is disabled due to restriction that trusted peer is down
    // 6. Both nodes are up

    let leader2 = controller.spawn_node(
        LEADER_2,
        LeadershipMode::Leader,
        PersistenceMode::Persistent,
    )?;

    leader2.wait_for_bootstrap()?;
    let leader1 = controller.spawn_node(
        LEADER_1,
        LeadershipMode::Leader,
        PersistenceMode::Persistent,
    )?;
    leader1.wait_for_bootstrap()?;

    utils::measure_and_log_sync_time(
        &[&leader1, &leader2],
        SyncWaitParams::nodes_restart(5).into(),
        "leader_leader_disruption_no_overlap",
        MeasurementReportInterval::Standard,
    )?;

    leader1.shutdown()?;
    leader2.shutdown()?;
    controller.finalize();
    Ok(ScenarioResult::passed(name))
}

#[named]
pub fn point_to_point_disruption(mut context: Context<ChaChaRng>) -> Result<ScenarioResult> {
    let name = function_name!();
    let scenario_settings = prepare_scenario! {
        name,
        &mut context,
        topology [
            LEADER_2,
            LEADER_1 -> LEADER_2,
            LEADER_3 -> LEADER_2
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

    let mut wallet1 = controller.wallet("unassigned1")?;
    let mut wallet2 = controller.wallet("delegated1")?;

    //monitor node disabled due to unsupported operation: restart node
    //controller.monitor_nodes();
    let leader2 = controller.spawn_node(
        LEADER_2,
        LeadershipMode::Leader,
        PersistenceMode::Persistent,
    )?;
    let leader1 = controller.spawn_node(
        LEADER_1,
        LeadershipMode::Leader,
        PersistenceMode::Persistent,
    )?;
    let leader3 = controller.spawn_node(
        LEADER_3,
        LeadershipMode::Leader,
        PersistenceMode::Persistent,
    )?;

    leader2.wait_for_bootstrap()?;
    leader1.wait_for_bootstrap()?;
    leader3.wait_for_bootstrap()?;

    controller.fragment_sender().send_transactions_round_trip(
        40,
        &mut wallet1,
        &mut wallet2,
        &leader1,
        1_000.into(),
    )?;

    leader2.shutdown()?;

    utils::measure_and_log_sync_time(
        &[&leader1, &leader3],
        SyncWaitParams::nodes_restart(5).into(),
        "point_to_point_disruption",
        MeasurementReportInterval::Standard,
    )?;

    leader3.shutdown()?;
    leader1.shutdown()?;
    controller.finalize();
    Ok(ScenarioResult::passed(name))
}

#[named]
pub fn point_to_point_disruption_overlap(
    mut context: Context<ChaChaRng>,
) -> Result<ScenarioResult> {
    let name = function_name!();
    let scenario_settings = prepare_scenario! {
        name,
        &mut context,
        topology [
            LEADER_2,
            LEADER_1 -> LEADER_2,
            LEADER_3 -> LEADER_2
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

    //monitor node disabled due to unsupported operation: restart node
    //controller.monitor_nodes();
    let mut leader2 = controller.spawn_node(
        LEADER_2,
        LeadershipMode::Leader,
        PersistenceMode::Persistent,
    )?;

    leader2.wait_for_bootstrap()?;
    let mut leader1 = controller.spawn_node(
        LEADER_1,
        LeadershipMode::Leader,
        PersistenceMode::Persistent,
    )?;
    leader1.wait_for_bootstrap()?;

    println!("1. 2 and 1 is up");
    utils::wait(5);

    println!("2. node 1 is down");
    leader1.shutdown()?;

    let mut leader3 = controller.spawn_node(
        LEADER_3,
        LeadershipMode::Leader,
        PersistenceMode::Persistent,
    )?;
    leader3.wait_for_bootstrap()?;

    println!("3. only Node 3 is up");
    leader2.shutdown()?;

    println!("4. 1 and 3 is up");
    leader1 = controller.spawn_node_custom(
        controller
            .new_spawn_params(LEADER_1)
            .leadership_mode(LeadershipMode::Leader)
            .persistence_mode(PersistenceMode::Persistent)
            .bootstrap_from_peers(false)
            .skip_bootstrap(true),
    )?;
    leader1.wait_for_bootstrap()?;

    println!("5. 2 and 3 is up");
    leader1.shutdown()?;
    leader2 = controller.spawn_node(
        LEADER_2,
        LeadershipMode::Leader,
        PersistenceMode::Persistent,
    )?;
    leader2.wait_for_bootstrap()?;

    println!("6. 1 and 2 is up");
    leader3.shutdown()?;

    leader1 = controller.spawn_node(
        LEADER_1,
        LeadershipMode::Leader,
        PersistenceMode::Persistent,
    )?;
    leader1.wait_for_bootstrap()?;

    println!("7. only Node 3 is up");
    leader1.shutdown()?;
    leader2.shutdown()?;

    leader3 = controller.spawn_node_custom(
        controller
            .new_spawn_params(LEADER_3)
            .leadership_mode(LeadershipMode::Leader)
            .persistence_mode(PersistenceMode::Persistent)
            .bootstrap_from_peers(false)
            .skip_bootstrap(true),
    )?;
    leader3.wait_for_bootstrap()?;

    println!("8. 1 and 3 is up");
    leader1 = controller.spawn_node_custom(
        controller
            .new_spawn_params(LEADER_1)
            .leadership_mode(LeadershipMode::Leader)
            .persistence_mode(PersistenceMode::Persistent)
            .bootstrap_from_peers(false)
            .skip_bootstrap(true),
    )?;
    leader1.wait_for_bootstrap()?;

    println!("9. all nodes are up");
    leader2 = controller.spawn_node(
        LEADER_2,
        LeadershipMode::Leader,
        PersistenceMode::Persistent,
    )?;
    leader2.wait_for_bootstrap()?;

    utils::measure_and_log_sync_time(
        &[&leader1, &leader2, &leader3],
        SyncWaitParams::nodes_restart(5).into(),
        "point_to_point_disruption_overlap",
        MeasurementReportInterval::Standard,
    )?;

    leader3.shutdown()?;
    leader2.shutdown()?;
    leader1.shutdown()?;
    controller.finalize();
    Ok(ScenarioResult::passed(name))
}

#[named]
pub fn custom_network_disruption(mut context: Context<ChaChaRng>) -> Result<ScenarioResult> {
    let name = function_name!();
    let scenario_settings = prepare_scenario! {
        name,
        &mut context,
        topology [
            LEADER_5,
            LEADER_1 -> LEADER_3,
            LEADER_2 -> LEADER_3 -> LEADER_5,
            LEADER_3 -> LEADER_5,
            LEADER_4 -> LEADER_5,
            PASSIVE -> LEADER_5,
        ]
        blockchain {
            consensus = GenesisPraos,
            number_of_slots_per_epoch = 60,
            slot_duration = 1,
            leaders = [ LEADER_1 ],
            initials = [
                "account" "unassigned1" with   500_000_000,
                "account" "delegated1" with  2_000_000_000 delegates to LEADER_1,
                "account" "delegated3" with  2_000_000_000 delegates to LEADER_3,
                "account" "delegated4" with  2_000_000_000 delegates to LEADER_4,
            ],
        }
    };

    let mut controller = scenario_settings.build(context)?;

    //monitor node disabled due to unsupported operation: restart node
    //controller.monitor_nodes();
    let leader5 = controller.spawn_node(
        LEADER_5,
        LeadershipMode::Leader,
        PersistenceMode::Persistent,
    )?;
    let leader4 = controller.spawn_node(
        LEADER_4,
        LeadershipMode::Leader,
        PersistenceMode::Persistent,
    )?;
    let leader3 = controller.spawn_node(
        LEADER_3,
        LeadershipMode::Leader,
        PersistenceMode::Persistent,
    )?;
    let leader2 = controller.spawn_node(
        LEADER_2,
        LeadershipMode::Leader,
        PersistenceMode::Persistent,
    )?;

    leader5.wait_for_bootstrap()?;
    leader4.wait_for_bootstrap()?;
    leader3.wait_for_bootstrap()?;
    leader2.wait_for_bootstrap()?;

    let mut wallet1 = controller.wallet("delegated1")?;
    let mut wallet3 = controller.wallet("delegated3")?;

    controller.fragment_sender().send_transactions_round_trip(
        2,
        &mut wallet1,
        &mut wallet3,
        &leader2,
        1_000.into(),
    )?;

    let leader1 = controller.spawn_node(
        LEADER_1,
        LeadershipMode::Leader,
        PersistenceMode::Persistent,
    )?;
    leader1.wait_for_bootstrap()?;

    controller.fragment_sender().send_transactions_round_trip(
        2,
        &mut wallet1,
        &mut wallet3,
        &leader3,
        1_000.into(),
    )?;

    leader2.shutdown()?;

    let passive = controller.spawn_node(
        PASSIVE,
        LeadershipMode::Passive,
        PersistenceMode::Persistent,
    )?;
    passive.wait_for_bootstrap()?;

    controller.fragment_sender().send_transactions_round_trip(
        2,
        &mut wallet1,
        &mut wallet3,
        &passive,
        1_000.into(),
    )?;

    utils::measure_and_log_sync_time(
        &[&leader1, &leader3, &leader4, &leader5, &passive],
        SyncWaitParams::nodes_restart(5).into(),
        "custom_network_disruption",
        MeasurementReportInterval::Standard,
    )?;

    passive.shutdown()?;
    leader5.shutdown()?;
    leader4.shutdown()?;
    leader3.shutdown()?;
    leader1.shutdown()?;
    controller.finalize();
    Ok(ScenarioResult::passed(name))
}

#[named]
pub fn mesh_disruption(mut context: Context<ChaChaRng>) -> Result<ScenarioResult> {
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

    //monitor node disabled due to unsupported operation: restart node
    //controller.monitor_nodes();
    let leader4 = controller.spawn_node(
        LEADER_4,
        LeadershipMode::Leader,
        PersistenceMode::Persistent,
    )?;
    leader4.wait_for_bootstrap()?;

    let mut leader5 = controller.spawn_node(
        LEADER_5,
        LeadershipMode::Leader,
        PersistenceMode::Persistent,
    )?;
    let leader3 = controller.spawn_node(
        LEADER_3,
        LeadershipMode::Leader,
        PersistenceMode::Persistent,
    )?;
    let mut leader2 = controller.spawn_node(
        LEADER_2,
        LeadershipMode::Leader,
        PersistenceMode::Persistent,
    )?;
    let leader1 = controller.spawn_node(
        LEADER_1,
        LeadershipMode::Leader,
        PersistenceMode::Persistent,
    )?;

    leader5.wait_for_bootstrap()?;
    leader3.wait_for_bootstrap()?;
    leader2.wait_for_bootstrap()?;
    leader1.wait_for_bootstrap()?;

    let mut wallet1 = controller.wallet("unassigned1")?;
    let mut wallet2 = controller.wallet("delegated1")?;

    controller.fragment_sender().send_transactions_round_trip(
        10,
        &mut wallet1,
        &mut wallet2,
        &leader1,
        1_000.into(),
    )?;

    leader2 =
        controller.restart_node(leader2, LeadershipMode::Leader, PersistenceMode::Persistent)?;

    controller.fragment_sender().send_transactions_round_trip(
        10,
        &mut wallet1,
        &mut wallet2,
        &leader1,
        1_000.into(),
    )?;

    leader5 =
        controller.restart_node(leader5, LeadershipMode::Leader, PersistenceMode::Persistent)?;

    controller.fragment_sender().send_transactions_round_trip(
        10,
        &mut wallet1,
        &mut wallet2,
        &leader1,
        1_000.into(),
    )?;

    utils::measure_and_log_sync_time(
        &[&leader1, &leader2, &leader3, &leader4, &leader5],
        SyncWaitParams::nodes_restart(5).into(),
        "mesh_disruption_sync",
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
