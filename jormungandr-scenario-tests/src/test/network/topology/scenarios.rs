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

const RELAY_NODE_1: &str = "Relay1";
const RELAY_NODE_2: &str = "Relay2";

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

pub fn relay(mut context: Context<ChaChaRng>) {
    let scenario_settings = prepare_scenario! {
        "T3007-Relay",
        &mut context,
        topology [
            RELAY_NODE_1 -> RELAY_NODE_2,
            RELAY_NODE_2 -> RELAY_NODE_1,
            LEADER_1 -> RELAY_NODE_1,RELAY_NODE_2,
            LEADER_2 -> RELAY_NODE_1,RELAY_NODE_2,
            LEADER_3 -> RELAY_NODE_1,RELAY_NODE_2,
            LEADER_4 -> RELAY_NODE_1,RELAY_NODE_2,
            LEADER_5 -> RELAY_NODE_1,RELAY_NODE_2,
            LEADER_6 -> RELAY_NODE_1,RELAY_NODE_2,
            LEADER_7 -> RELAY_NODE_1,RELAY_NODE_2
        ]
        blockchain {
            consensus = GenesisPraos,
            number_of_slots_per_epoch = 60,
            slot_duration = 1,
            leaders = [ LEADER_1 ],
            initials = [
                account "delegated1" with  1_000_000_000 delegates to LEADER_1,
                account "delegated2" with  1_000_000_000 delegates to LEADER_2,
                account "delegated3" with  1_000_000_000 delegates to LEADER_3,
                account "delegated4" with  1_000_000_000 delegates to LEADER_4,
                account "delegated5" with  1_000_000_000 delegates to LEADER_5,
                account "delegated6" with  1_000_000_000 delegates to LEADER_6,
                account "delegated7" with  1_000_000_000 delegates to LEADER_7,
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
    let relay1 = controller.spawn_leader_node(RELAY_NODE_1, false).unwrap();
    let relay2 = controller.spawn_leader_node(RELAY_NODE_2, false).unwrap();

    controller.monitor_nodes();

    // let arbitrary_node = ArbitraryNodeController::prepare(vec![leader1.clone()], &mut context);

    thread::sleep(Duration::from_secs(10));

    let mut wallet1 = controller.wallet("delegated1").unwrap();
    let mut wallet2 = controller.wallet("delegated2").unwrap();
    let mut wallet3 = controller.wallet("delegated3").unwrap();
    let mut wallet4 = controller.wallet("delegated4").unwrap();
    let mut wallet5 = controller.wallet("delegated5").unwrap();
    let mut wallet6 = controller.wallet("delegated6").unwrap();
    let mut wallet7 = controller.wallet("delegated7").unwrap();

    for _ in 0..200000 {
        let check1 = controller
            .wallet_send_to(&mut wallet1, &wallet2, &leader1, 1_000.into())
            .unwrap();

        let check2 = controller
            .wallet_send_to(&mut wallet2, &wallet1, &leader2, 1_000.into())
            .unwrap();

        let check3 = controller
            .wallet_send_to(&mut wallet3, &wallet4, &leader3, 1_000.into())
            .unwrap();

        let check4 = controller
            .wallet_send_to(&mut wallet4, &wallet3, &leader4, 1_000.into())
            .unwrap();

        let check5 = controller
            .wallet_send_to(&mut wallet5, &wallet6, &leader5, 1_000.into())
            .unwrap();

        let check6 = controller
            .wallet_send_to(&mut wallet6, &wallet1, &leader6, 1_000.into())
            .unwrap();

        let check7 = controller
            .wallet_send_to(&mut wallet7, &wallet6, &leader7, 1_000.into())
            .unwrap();

        let mut statuses = Vec::new();
        statuses.push(leader1.wait_fragment(Duration::from_secs(2), check1));
        statuses.push(leader2.wait_fragment(Duration::from_secs(2), check2));
        statuses.push(leader3.wait_fragment(Duration::from_secs(2), check3));
        statuses.push(leader4.wait_fragment(Duration::from_secs(2), check4));
        statuses.push(leader5.wait_fragment(Duration::from_secs(2), check5));
        statuses.push(leader6.wait_fragment(Duration::from_secs(2), check6));
        statuses.push(leader7.wait_fragment(Duration::from_secs(2), check7));

        if statuses.iter().any(|x| x.is_err()) {
            break;
        }

        if statuses
            .iter()
            .any(|x| !x.as_ref().unwrap().is_in_a_block())
        {
            break;
        }

        wallet1.confirm_transaction();
        wallet2.confirm_transaction();
        wallet3.confirm_transaction();
        wallet4.confirm_transaction();
        wallet5.confirm_transaction();
        wallet6.confirm_transaction();
        wallet7.confirm_transaction();
    }

    leader7.shutdown().unwrap();
    leader6.shutdown().unwrap();
    leader5.shutdown().unwrap();
    leader4.shutdown().unwrap();
    leader3.shutdown().unwrap();
    leader2.shutdown().unwrap();
    leader1.shutdown().unwrap();

    relay1.shutdown().unwrap();
    relay2.shutdown().unwrap();

    controller.finalize();
}
