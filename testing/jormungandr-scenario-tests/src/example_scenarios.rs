#![allow(dead_code)]

use crate::{
    node::{LeadershipMode, PersistenceMode},
    scenario::repository::ScenarioResult,
    test::Result,
    Context,
};
use function_name::named;
use rand_chacha::ChaChaRng;

#[named]
pub fn scenario_1(mut context: Context<ChaChaRng>) -> Result<ScenarioResult> {
    let name = function_name!();
    let scenario_settings = prepare_scenario! {
        name,
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
                "account" "faucet1" with 1_000_000_000,
                "account" "faucet2" with 2_000_000_000 delegates to "node2",
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
    let _block = node2.rest().block(&tip1.into_hash())?;

    std::thread::sleep(std::time::Duration::from_secs(1));

    node2.shutdown()?;
    controller.finalize();
    Ok(ScenarioResult::passed(name))
}

#[named]
pub fn scenario_2(mut context: Context<ChaChaRng>) -> Result<ScenarioResult> {
    let name = function_name!();
    let scenario_settings = prepare_scenario! {
        name,
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
                "account" "unassigned1" with   500_000_000,
                "account" "unassigned2" with   100_000_000,
                "account" "delegated1" with  2_000_000_000 delegates to "Leader1",
                "account" "delegated2" with    300_000_000 delegates to "Unknown1",
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
    let mut wallet2 = controller.wallet("delegated1").unwrap();

    controller.fragment_sender().send_transactions_round_trip(
        10,
        &mut wallet1,
        &mut wallet2,
        &leader1,
        1_000.into(),
    )?;

    leader1.shutdown()?;
    passive1.shutdown()?;
    passive2.shutdown()?;
    passive3.shutdown()?;
    controller.finalize();
    Ok(ScenarioResult::passed(name))
}
