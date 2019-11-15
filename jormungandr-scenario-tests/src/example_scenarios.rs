use crate::{
    node::{LeadershipMode, PersistenceMode},
    Context,
};
use rand_chacha::ChaChaRng;
use std::{thread, time::Duration};

pub fn scenario_1(mut context: Context<ChaChaRng>) {
    let scenario_settings = prepare_scenario! {
        "simple network example",
        &mut context,
        topology [
            "node1",
            "node2" -> "node1",
        ]
        blockchain {
            consensus = Bft,
            number_of_slots_per_epoch = 10,
            slot_duration = 1,
            leaders = [ "node1" ],
            initials = [
                account "faucet1" with 1_000_000_000,
                account "faucet2" with 2_000_000_000 delegates to "node2",
            ],
        }
    };

    let mut controller = scenario_settings.build(context).unwrap();

    let node1 = controller
        .spawn_node("node1", LeadershipMode::Leader, PersistenceMode::InMemory)
        .unwrap();
    let node2 = controller
        .spawn_node("node2", LeadershipMode::Passive, PersistenceMode::InMemory)
        .unwrap();

    controller.monitor_nodes();
    std::thread::sleep(std::time::Duration::from_secs(10));
    let tip1 = node1.tip().unwrap();
    std::thread::sleep(std::time::Duration::from_secs(1));
    node1.shutdown().unwrap();
    let _block = node2.block(&tip1).unwrap();

    std::thread::sleep(std::time::Duration::from_secs(1));

    node2.shutdown().unwrap();

    controller.finalize();
}

pub fn scenario_2(mut context: Context<ChaChaRng>) {
    let scenario_settings = prepare_scenario! {
        "Testing the network",
        &mut context,
        topology [
            "Leader1",
            "Passive1" -> "Leader1",
            "Passive2" -> "Leader1",
            "Passive3" -> "Leader1",
            "Unknown1",
        ]
        blockchain {
            consensus = GenesisPraos,
            number_of_slots_per_epoch = 60,
            slot_duration = 1,
            leaders = [ "Leader2" ],
            initials = [
                account "unassigned1" with   500_000_000,
                account "unassigned2" with   100_000_000,
                account "delegated1" with  2_000_000_000 delegates to "Leader1",
                account "delegated2" with    300_000_000 delegates to "Unknown1",
            ],
        }
    };

    let mut controller = scenario_settings.build(context).unwrap();

    let leader1 = controller
        .spawn_node("Leader1", LeadershipMode::Leader, PersistenceMode::InMemory)
        .unwrap();
    thread::sleep(Duration::from_secs(2));
    let passive1 = controller
        .spawn_node(
            "Passive1",
            LeadershipMode::Passive,
            PersistenceMode::InMemory,
        )
        .unwrap();
    let _passive2 = controller
        .spawn_node(
            "Passive2",
            LeadershipMode::Passive,
            PersistenceMode::InMemory,
        )
        .unwrap();
    let _passive3 = controller
        .spawn_node(
            "Passive3",
            LeadershipMode::Passive,
            PersistenceMode::InMemory,
        )
        .unwrap();

    controller.monitor_nodes();

    let mut wallet1 = controller.wallet("unassigned1").unwrap();
    let wallet2 = controller.wallet("delegated1").unwrap();

    for i in 0..10 {
        let check = controller.wallet_send_to(&mut wallet1, &wallet2, &leader1, 5_000.into())?;

        thread::sleep(Duration::from_secs(1));

        let status = leader1.wait_fragment(Duration::from_secs(2), check);

        if let Ok(status) = status {
            if status.is_in_a_block() {
                wallet1.confirm_transaction();
            } else {
                break;
            }
        } else {
            break;
        }
    }

    leader1.shutdown().unwrap();
    passive1.shutdown().unwrap();

    controller.finalize();
}
