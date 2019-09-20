use crate::{
    node::NodeController,
    scenario::Controller,
    test::utils::{self, ArbitraryNodeController},
    wallet::Wallet,
    Context,
};

use rand_chacha::ChaChaRng;
use std::{thread, time::Duration};

const LEADER_1: &str = "Leader1";
const LEADER_2: &str = "Leader2";
const LEADER_3: &str = "Leader3";
const LEADER_4: &str = "Leader4";
const LEADER_5: &str = "Leader5";
const LEADER_6: &str = "Leader6";
const LEADER_7: &str = "Leader7";

pub fn fully_connected(mut context: Context<ChaChaRng>) {
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
                account "unassigned1" with   500_000_000,
                account "delegated1" with  2_000_000_000 delegates to LEADER_2,
            ],
        }
    };

    let mut controller = scenario_settings.build(context).unwrap();

    controller.monitor_nodes();
    let leader4 = controller.spawn_leader_node(LEADER_4, false).unwrap();
    let leader3 = controller.spawn_leader_node(LEADER_3, false).unwrap();
    let leader2 = controller.spawn_leader_node(LEADER_2, false).unwrap();
    let leader1 = controller.spawn_leader_node(LEADER_1, false).unwrap();

    thread::sleep(Duration::from_secs(10));

    let mut wallet1 = controller.wallet("unassigned1").unwrap();
    let wallet2 = controller.wallet("delegated1").unwrap();

    utils::keep_sending_transaction_to_node_until_error(
        &mut controller,
        &mut wallet1,
        &wallet2,
        &leader1,
    );

    leader4.shutdown().unwrap();
    leader3.shutdown().unwrap();
    leader2.shutdown().unwrap();
    leader1.shutdown().unwrap();

    controller.finalize();
}

pub fn star(mut context: Context<ChaChaRng>) {
    let scenario_settings = prepare_scenario! {
        "T3002_Star",
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
                account "unassigned1" with   500_000_000,
                account "delegated1" with  2_000_000_000 delegates to LEADER_5,
            ],
        }
    };

    let mut controller = scenario_settings.build(context).unwrap();

    controller.monitor_nodes();
    let leader5 = controller.spawn_leader_node(LEADER_5, false).unwrap();
    let leader4 = controller.spawn_leader_node(LEADER_4, false).unwrap();
    let leader3 = controller.spawn_leader_node(LEADER_3, false).unwrap();
    let leader2 = controller.spawn_leader_node(LEADER_2, false).unwrap();
    let leader1 = controller.spawn_leader_node(LEADER_1, false).unwrap();

    thread::sleep(Duration::from_secs(10));

    let mut wallet1 = controller.wallet("unassigned1").unwrap();
    let wallet2 = controller.wallet("delegated1").unwrap();

    utils::keep_sending_transaction_to_node_until_error(
        &mut controller,
        &mut wallet1,
        &wallet2,
        &leader1,
    );

    leader5.shutdown().unwrap();
    leader4.shutdown().unwrap();
    leader3.shutdown().unwrap();
    leader2.shutdown().unwrap();
    leader1.shutdown().unwrap();

    controller.finalize();
}

pub fn ring(mut context: Context<ChaChaRng>) {
    let scenario_settings = prepare_scenario! {
        "T3003_Ring",
        &mut context,
        topology [
            LEADER_1 -> LEADER_2,
            LEADER_2 -> LEADER_3,
            LEADER_3 -> LEADER_4,
            LEADER_4 -> LEADER_1,
        ]
        blockchain {
            consensus = GenesisPraos,
            number_of_slots_per_epoch = 60,
            slot_duration = 1,
            leaders = [ LEADER_1 ],
            initials = [
                account "unassigned1" with   500_000_000,
                account "delegated1" with  2_000_000_000 delegates to LEADER_5,
            ],
        }
    };

    let mut controller = scenario_settings.build(context).unwrap();

    controller.monitor_nodes();
    let leader4 = controller.spawn_leader_node(LEADER_4, false).unwrap();
    let leader3 = controller.spawn_leader_node(LEADER_3, false).unwrap();
    let leader2 = controller.spawn_leader_node(LEADER_2, false).unwrap();
    let leader1 = controller.spawn_leader_node(LEADER_1, false).unwrap();

    thread::sleep(Duration::from_secs(10));

    let mut wallet1 = controller.wallet("unassigned1").unwrap();
    let wallet2 = controller.wallet("delegated1").unwrap();

    utils::keep_sending_transaction_to_node_until_error(
        &mut controller,
        &mut wallet1,
        &wallet2,
        &leader1,
    );

    leader4.shutdown().unwrap();
    leader3.shutdown().unwrap();
    leader2.shutdown().unwrap();
    leader1.shutdown().unwrap();

    controller.finalize();
}

pub fn mesh(mut context: Context<ChaChaRng>) {
    let scenario_settings = prepare_scenario! {
        "T3004_Mesh",
        &mut context,
        topology [
            LEADER_1 -> LEADER_4,LEADER_5,
            LEADER_2 -> LEADER_1,LEADER_3,
            LEADER_3 -> LEADER_1,LEADER_4,
            LEADER_4 -> LEADER_5,
            LEADER_5 -> LEADER_3,LEADER_1,
        ]
        blockchain {
            consensus = GenesisPraos,
            number_of_slots_per_epoch = 60,
            slot_duration = 1,
            leaders = [ LEADER_1 ],
            initials = [
                account "unassigned1" with   500_000_000,
                account "delegated1" with  2_000_000_000 delegates to LEADER_3,
            ],
        }
    };

    let mut controller = scenario_settings.build(context).unwrap();

    controller.monitor_nodes();
    let leader5 = controller.spawn_leader_node(LEADER_5, false).unwrap();
    let leader4 = controller.spawn_leader_node(LEADER_4, false).unwrap();
    let leader3 = controller.spawn_leader_node(LEADER_3, false).unwrap();
    let leader2 = controller.spawn_leader_node(LEADER_2, false).unwrap();
    let leader1 = controller.spawn_leader_node(LEADER_1, false).unwrap();

    thread::sleep(Duration::from_secs(10));

    let mut wallet1 = controller.wallet("unassigned1").unwrap();
    let wallet2 = controller.wallet("delegated1").unwrap();

    utils::keep_sending_transaction_to_node_until_error(
        &mut controller,
        &mut wallet1,
        &wallet2,
        &leader1,
    );

    leader5.shutdown().unwrap();
    leader4.shutdown().unwrap();
    leader3.shutdown().unwrap();
    leader2.shutdown().unwrap();
    leader1.shutdown().unwrap();
}

pub fn point_to_point(mut context: Context<ChaChaRng>) {
    let scenario_settings = prepare_scenario! {
        "T3005-Point-to-Point",
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
                account "unassigned1" with   500_000_000,
                account "delegated1" with  2_000_000_000 delegates to LEADER_1,
            ],
        }
    };

    let mut controller = scenario_settings.build(context).unwrap();

    controller.monitor_nodes();
    let leader4 = controller.spawn_leader_node(LEADER_4, false).unwrap();
    let leader3 = controller.spawn_leader_node(LEADER_3, false).unwrap();
    let leader2 = controller.spawn_leader_node(LEADER_2, false).unwrap();
    let leader1 = controller.spawn_leader_node(LEADER_1, false).unwrap();

    thread::sleep(Duration::from_secs(10));

    let mut wallet1 = controller.wallet("unassigned1").unwrap();
    let wallet2 = controller.wallet("delegated1").unwrap();

    utils::keep_sending_transaction_to_node_until_error(
        &mut controller,
        &mut wallet1,
        &wallet2,
        &leader1,
    );

    leader4.shutdown().unwrap();
    leader3.shutdown().unwrap();
    leader2.shutdown().unwrap();
    leader1.shutdown().unwrap();

    controller.finalize();
}

pub fn tree(mut context: Context<ChaChaRng>) {
    let scenario_settings = prepare_scenario! {
        "T3006-Tree",
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
                account "unassigned1" with   500_000_000,
                account "delegated1" with  2_000_000_000 delegates to LEADER_7,
            ],
        }
    };

    let mut controller = scenario_settings.build(context.clone()).unwrap();

    let leader1 = controller.spawn_leader_node(LEADER_1, false).unwrap();
    let leader2 = controller.spawn_leader_node(LEADER_2, false).unwrap();
    let leader3 = controller.spawn_leader_node(LEADER_3, false).unwrap();
    let leader4 = controller.spawn_leader_node(LEADER_4, false).unwrap();
    let leader5 = controller.spawn_leader_node(LEADER_5, false).unwrap();
    let leader6 = controller.spawn_leader_node(LEADER_6, false).unwrap();
    let leader7 = controller.spawn_leader_node(LEADER_7, false).unwrap();

    controller.monitor_nodes();

    // let arbitrary_node = ArbitraryNodeController::prepare(vec![leader1.clone()], &mut context);

    thread::sleep(Duration::from_secs(10));

    let mut wallet1 = controller.wallet("unassigned1").unwrap();
    let wallet2 = controller.wallet("delegated1").unwrap();

    utils::keep_sending_transaction_to_node_until_error(
        &mut controller,
        &mut wallet1,
        &wallet2,
        &leader1,
    );

    leader7.shutdown().unwrap();
    leader6.shutdown().unwrap();
    leader5.shutdown().unwrap();
    leader4.shutdown().unwrap();
    leader3.shutdown().unwrap();
    leader2.shutdown().unwrap();
    leader1.shutdown().unwrap();

    controller.finalize();
}
