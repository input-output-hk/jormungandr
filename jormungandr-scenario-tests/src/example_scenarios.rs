use crate::{
    node::{LeadershipMode, PersistenceMode},
    test::Result,
    Context, ScenarioResult,
};
use rand_chacha::ChaChaRng;
use std::{thread, time::Duration};

pub fn scenario_1(mut context: Context<ChaChaRng>) -> Result<ScenarioResult> {
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

    let mut controller = scenario_settings.build(context)?;

    let node1 =
        controller.spawn_node("node1", LeadershipMode::Leader, PersistenceMode::InMemory)?;
    let node2 =
        controller.spawn_node("node2", LeadershipMode::Passive, PersistenceMode::InMemory)?;

    controller.monitor_nodes();

    node1.wait_for_bootstrap()?;
    node2.wait_for_bootstrap()?;

    let tip1 = node1.tip()?;
    std::thread::sleep(std::time::Duration::from_secs(1));
    node1.shutdown()?;
    let _block = node2.block(&tip1)?;

    std::thread::sleep(std::time::Duration::from_secs(1));

    node2.shutdown()?;
    controller.finalize();
    Ok(ScenarioResult::Passed)
}

pub fn scenario_2(mut context: Context<ChaChaRng>) -> Result<ScenarioResult> {
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

    let leader1 =
        controller.spawn_node("Leader1", LeadershipMode::Leader, PersistenceMode::InMemory)?;
    let passive1 = controller.spawn_node(
        "Passive1",
        LeadershipMode::Passive,
        PersistenceMode::InMemory,
    )?;
    let passive2 = controller.spawn_node(
        "Passive2",
        LeadershipMode::Passive,
        PersistenceMode::InMemory,
    )?;
    let passive3 = controller.spawn_node(
        "Passive3",
        LeadershipMode::Passive,
        PersistenceMode::InMemory,
    )?;

    controller.monitor_nodes();

    leader1.wait_for_bootstrap()?;
    passive1.wait_for_bootstrap()?;
    passive2.wait_for_bootstrap()?;
    passive3.wait_for_bootstrap()?;

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
                return Ok(ScenarioResult::Failed(format!(
                    "transaction no. {} not confirmed",
                    i
                )));
            }
        } else {
            return Ok(ScenarioResult::Failed(format!(
                "cannot get status from leader1"
            )));
        }
    }

    leader1.shutdown()?;
    passive1.shutdown()?;
    passive2.shutdown()?;
    passive3.shutdown()?;
    controller.finalize();
    Ok(ScenarioResult::Passed)
}
