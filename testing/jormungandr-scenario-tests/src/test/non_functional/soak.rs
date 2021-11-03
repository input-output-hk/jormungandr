use crate::test::non_functional::*;
use crate::{
    node::{LeadershipMode, PersistenceMode},
    scenario::repository::ScenarioResult,
    test::utils::SyncWaitParams,
    test::Result,
    Context,
};
use jormungandr_testing_utils::testing::{ensure_nodes_are_in_sync, FragmentVerifier};
use rand_chacha::ChaChaRng;
use std::time::{Duration, SystemTime};

const CORE_NODE: &str = "Core";
const RELAY_NODE_1: &str = "Relay1";
const RELAY_NODE_2: &str = "Relay2";
use function_name::named;

#[named]
pub fn relay_soak(mut context: Context<ChaChaRng>) -> Result<ScenarioResult> {
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
            slot_duration = 10,
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
    let mut wallet3 = controller.wallet("delegated3")?;
    let mut wallet4 = controller.wallet("delegated4")?;
    let mut wallet5 = controller.wallet("delegated5")?;
    let mut wallet6 = controller.wallet("delegated6")?;
    let mut wallet7 = controller.wallet("delegated7")?;

    let now = SystemTime::now();

    let fragment_sender = controller.fragment_sender();

    loop {
        let check1 =
            fragment_sender.send_transaction(&mut wallet1, &wallet2, &leader1, 1_000.into())?;
        let check2 =
            fragment_sender.send_transaction(&mut wallet2, &wallet1, &leader2, 1_000.into())?;
        let check3 =
            fragment_sender.send_transaction(&mut wallet3, &wallet4, &leader3, 1_000.into())?;
        let check4 =
            fragment_sender.send_transaction(&mut wallet4, &wallet3, &leader4, 1_000.into())?;
        let check5 =
            fragment_sender.send_transaction(&mut wallet5, &wallet6, &leader5, 1_000.into())?;
        let check6 =
            fragment_sender.send_transaction(&mut wallet6, &wallet1, &leader6, 1_000.into())?;
        let check7 =
            fragment_sender.send_transaction(&mut wallet7, &wallet6, &leader7, 1_000.into())?;

        FragmentVerifier::wait_and_verify_is_in_block(Duration::from_secs(2), check1, &leader1)?;
        FragmentVerifier::wait_and_verify_is_in_block(Duration::from_secs(2), check2, &leader2)?;
        FragmentVerifier::wait_and_verify_is_in_block(Duration::from_secs(2), check3, &leader3)?;
        FragmentVerifier::wait_and_verify_is_in_block(Duration::from_secs(2), check4, &leader4)?;
        FragmentVerifier::wait_and_verify_is_in_block(Duration::from_secs(2), check5, &leader5)?;
        FragmentVerifier::wait_and_verify_is_in_block(Duration::from_secs(2), check6, &leader6)?;
        FragmentVerifier::wait_and_verify_is_in_block(Duration::from_secs(2), check7, &leader7)?;

        wallet1.confirm_transaction();
        wallet2.confirm_transaction();
        wallet3.confirm_transaction();
        wallet4.confirm_transaction();
        wallet5.confirm_transaction();
        wallet6.confirm_transaction();
        wallet7.confirm_transaction();

        // 48 hours
        if now.elapsed().unwrap().as_secs() > (900) {
            break;
        }
    }

    ensure_nodes_are_in_sync(
        SyncWaitParams::ZeroWait,
        &[
            &leader1, &leader2, &leader3, &leader4, &leader5, &leader6, &leader7, &relay1, &relay2,
        ],
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
