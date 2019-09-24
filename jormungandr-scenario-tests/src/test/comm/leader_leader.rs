use crate::{test::utils, Context};
use rand_chacha::ChaChaRng;
use std::{thread, time::Duration};

const LEADER_1: &str = "Leader1";
const LEADER_2: &str = "Leader2";

pub fn two_transaction_to_two_leaders(mut context: Context<ChaChaRng>) {
    let scenario_settings = prepare_scenario! {
        "L2101-Leader_to_leader_communication",
        &mut context,
        topology [
            LEADER_1 -> LEADER_2,
            LEADER_2 -> LEADER_1,
        ]
        blockchain {
            consensus = GenesisPraos,
            number_of_slots_per_epoch = 60,
            slot_duration = 1,
            leaders = [ LEADER_1 ],
            initials = [
                account "delegated1" with  2_500_000_000 delegates to LEADER_2,
                account "delegated2" with  2_000_000_000 delegates to LEADER_1,
            ],
        }
    };

    let mut controller = scenario_settings.build(context).unwrap();

    controller.monitor_nodes();
    let leader_1 = controller.spawn_leader_node(LEADER_1, false).unwrap();
    let leader_2 = controller.spawn_leader_node(LEADER_2, false).unwrap();

    thread::sleep(Duration::from_secs(4));

    let mut wallet1 = controller.wallet("delegated2").unwrap();
    let mut wallet2 = controller.wallet("delegated1").unwrap();

    for _ in 0..200000 {
        let check1 = controller
            .wallet_send_to(&mut wallet1, &wallet2, &leader_1, 1_000.into())
            .unwrap();

        let check2 = controller
            .wallet_send_to(&mut wallet2, &wallet1, &leader_2, 1_000.into())
            .unwrap();

        let status_1 = leader_1.wait_fragment(Duration::from_secs(2), check1);
        let status_2 = leader_2.wait_fragment(Duration::from_secs(2), check2);

        if status_1.is_err() || status_2.is_err() {
            break;
        }

        let status_1 = status_1.unwrap();
        let status_2 = status_2.unwrap();

        if !status_1.is_in_a_block() || !status_2.is_in_a_block() {
            break;
        }

        wallet1.confirm_transaction();
        wallet2.confirm_transaction();
    }

    leader_1.shutdown().unwrap();
    leader_2.shutdown().unwrap();
    controller.finalize();
}
