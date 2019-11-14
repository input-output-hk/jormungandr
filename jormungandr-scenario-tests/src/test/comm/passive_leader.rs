use crate::{
    node::{LeadershipMode, PersistenceMode},
    test::utils,
    Context,
};
use rand_chacha::ChaChaRng;
use std::{thread, time::Duration};

const LEADER: &str = "Leader";
const PASSIVE: &str = "Passive";

pub fn transaction_to_passive(mut context: Context<ChaChaRng>) {
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

    let mut controller = scenario_settings.build(context).unwrap();

    controller.monitor_nodes();
    let leader = controller
        .spawn_node(LEADER, LeadershipMode::Leader, PersistenceMode::InMemory)
        .unwrap();
    let passive = controller
        .spawn_node(PASSIVE, LeadershipMode::Passive, PersistenceMode::InMemory)
        .unwrap();
    thread::sleep(Duration::from_secs(4));

    let mut wallet1 = controller.wallet("unassigned1").unwrap();
    let wallet2 = controller.wallet("delegated1").unwrap();

    utils::sending_transactions_to_node_sequentially(
        20,
        &mut controller,
        &mut wallet1,
        &wallet2,
        &passive,
    );

    utils::assert_are_in_sync(vec![&passive, &leader]);

    passive.shutdown().unwrap();
    leader.shutdown().unwrap();
    controller.finalize();
}

pub fn leader_is_offline(mut context: Context<ChaChaRng>) {
    let scenario_settings = prepare_scenario! {
        "L2002-leader_is_offline_while_passive_receives_tx",
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

    let mut controller = scenario_settings.build(context).unwrap();

    controller.monitor_nodes();
    let passive = controller
        .spawn_node(PASSIVE, LeadershipMode::Passive, PersistenceMode::InMemory)
        .unwrap();

    thread::sleep(Duration::from_secs(2));

    let mut wallet1 = controller.wallet("unassigned1").unwrap();
    let wallet2 = controller.wallet("delegated1").unwrap();

    utils::keep_sending_transaction_dispite_error(
        40,
        &mut controller,
        &mut wallet1,
        &wallet2,
        &passive,
    );

    passive.shutdown().unwrap();

    controller.finalize();
}

pub fn leader_is_online_with_delay(mut context: Context<ChaChaRng>) {
    let scenario_settings = prepare_scenario! {
        "L2003-leader_is_online_with_delay",
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

    let mut controller = scenario_settings.build(context).unwrap();

    controller.monitor_nodes();
    let passive = controller
        .spawn_node(PASSIVE, LeadershipMode::Passive, PersistenceMode::InMemory)
        .unwrap();

    thread::sleep(Duration::from_secs(3));

    let mut wallet1 = controller.wallet("unassigned1").unwrap();
    let wallet2 = controller.wallet("delegated1").unwrap();

    utils::keep_sending_transaction_dispite_error(
        10,
        &mut controller,
        &mut wallet1,
        &wallet2,
        &passive,
    );

    let leader = controller
        .spawn_node(LEADER, LeadershipMode::Leader, PersistenceMode::InMemory)
        .unwrap();
    thread::sleep(Duration::from_secs(3));

    utils::keep_sending_transaction_dispite_error(
        40,
        &mut controller,
        &mut wallet1,
        &wallet2,
        &passive,
    );

    utils::assert_are_in_sync(vec![&passive, &leader]);

    passive.shutdown().unwrap();
    leader.shutdown().unwrap();

    controller.finalize();
}

pub fn leader_restart(mut context: Context<ChaChaRng>) {
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

    let mut controller = scenario_settings.build(context).unwrap();

    controller.monitor_nodes();
    let passive = controller
        .spawn_node(PASSIVE, LeadershipMode::Passive, PersistenceMode::InMemory)
        .unwrap();
    let leader = controller
        .spawn_node(LEADER, LeadershipMode::Leader, PersistenceMode::InMemory)
        .unwrap();

    thread::sleep(Duration::from_secs(10));

    let mut wallet1 = controller.wallet("unassigned1").unwrap();
    let wallet2 = controller.wallet("delegated1").unwrap();

    utils::sending_transactions_to_node_sequentially(
        10,
        &mut controller,
        &mut wallet1,
        &wallet2,
        &passive,
    );

    leader.shutdown().unwrap();

    utils::sending_transactions_to_node_sequentially(
        10,
        &mut controller,
        &mut wallet1,
        &wallet2,
        &passive,
    );

    let leader = controller
        .spawn_node(LEADER, LeadershipMode::Leader, PersistenceMode::InMemory)
        .unwrap();

    utils::keep_sending_transaction_to_node_until_error(
        40,
        &mut controller,
        &mut wallet1,
        &wallet2,
        &passive,
    );

    utils::assert_are_in_sync(vec![&passive, &leader]);

    passive.shutdown().unwrap();
    leader.shutdown().unwrap();

    controller.finalize();
}

pub fn passive_node_is_updated(mut context: Context<ChaChaRng>) {
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

    let mut controller = scenario_settings.build(context).unwrap();

    controller.monitor_nodes();
    let passive = controller
        .spawn_node(PASSIVE, LeadershipMode::Passive, PersistenceMode::InMemory)
        .unwrap();
    let leader = controller
        .spawn_node(LEADER, LeadershipMode::Leader, PersistenceMode::InMemory)
        .unwrap();

    thread::sleep(Duration::from_secs(10));

    let mut wallet1 = controller.wallet("unassigned1").unwrap();
    let wallet2 = controller.wallet("delegated1").unwrap();

    utils::keep_sending_transaction_to_node_until_error(
        40,
        &mut controller,
        &mut wallet1,
        &wallet2,
        &leader,
    );

    utils::assert_are_in_sync(vec![&passive, &leader]);

    passive.shutdown().unwrap();
    leader.shutdown().unwrap();

    controller.finalize();
}
