use crate::{
    node::{LeadershipMode, PersistenceMode},
    scenario::repository::ScenarioResult,
    test::utils::{self, SyncWaitParams},
    test::Result,
    Context,
};
use rand_chacha::ChaChaRng;
use std::time::{Duration, SystemTime};

use crate::test::non_functional::*;

const CORE_NODE: &str = "Core";
const RELAY_NODE_1: &str = "Relay1";
const RELAY_NODE_2: &str = "Relay2";

pub fn relay_soak(mut context: Context<ChaChaRng>) -> Result<ScenarioResult> {
    let scenario_settings = prepare_scenario! {
        "Relay-Soak",
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

    let mut controller = scenario_settings.build(context.clone())?;

    let core =
        controller.spawn_node(CORE_NODE, LeadershipMode::Leader, PersistenceMode::InMemory)?;

    controller.monitor_nodes();
    core.wait_for_bootstrap()?;

    let relay1 = controller.spawn_node(
        RELAY_NODE_1,
        LeadershipMode::Passive,
        PersistenceMode::InMemory,
    )?;
    let relay2 = controller.spawn_node(
        RELAY_NODE_2,
        LeadershipMode::Passive,
        PersistenceMode::InMemory,
    )?;

    relay2.wait_for_bootstrap()?;
    relay1.wait_for_bootstrap()?;

    let leader1 =
        controller.spawn_node(LEADER_1, LeadershipMode::Leader, PersistenceMode::InMemory)?;
    let leader2 =
        controller.spawn_node(LEADER_2, LeadershipMode::Leader, PersistenceMode::InMemory)?;
    let leader3 =
        controller.spawn_node(LEADER_3, LeadershipMode::Leader, PersistenceMode::InMemory)?;
    let leader4 =
        controller.spawn_node(LEADER_4, LeadershipMode::Leader, PersistenceMode::InMemory)?;
    let leader5 =
        controller.spawn_node(LEADER_5, LeadershipMode::Leader, PersistenceMode::InMemory)?;
    let leader6 =
        controller.spawn_node(LEADER_6, LeadershipMode::Leader, PersistenceMode::InMemory)?;
    let leader7 =
        controller.spawn_node(LEADER_7, LeadershipMode::Leader, PersistenceMode::InMemory)?;

    leader7.wait_for_bootstrap()?;
    leader6.wait_for_bootstrap()?;
    leader5.wait_for_bootstrap()?;
    leader4.wait_for_bootstrap()?;
    leader3.wait_for_bootstrap()?;
    leader2.wait_for_bootstrap()?;
    leader1.wait_for_bootstrap()?;

    let mut wallet1 = controller.wallet("delegated1")?;
    let mut wallet2 = controller.wallet("delegated2")?;
    let mut wallet3 = controller.wallet("delegated3")?;
    let mut wallet4 = controller.wallet("delegated4")?;
    let mut wallet5 = controller.wallet("delegated5")?;
    let mut wallet6 = controller.wallet("delegated6")?;
    let mut wallet7 = controller.wallet("delegated7")?;

    let now = SystemTime::now();

    loop {
        let check1 = controller.wallet_send_to(&mut wallet1, &wallet2, &leader1, 1_000.into())?;
        let check2 = controller.wallet_send_to(&mut wallet2, &wallet1, &leader2, 1_000.into())?;
        let check3 = controller.wallet_send_to(&mut wallet3, &wallet4, &leader3, 1_000.into())?;
        let check4 = controller.wallet_send_to(&mut wallet4, &wallet3, &leader4, 1_000.into())?;
        let check5 = controller.wallet_send_to(&mut wallet5, &wallet6, &leader5, 1_000.into())?;
        let check6 = controller.wallet_send_to(&mut wallet6, &wallet1, &leader6, 1_000.into())?;
        let check7 = controller.wallet_send_to(&mut wallet7, &wallet6, &leader7, 1_000.into())?;

        let status1 = leader1.wait_fragment(Duration::from_secs(2), check1)?;
        let status2 = leader2.wait_fragment(Duration::from_secs(2), check2)?;
        let status3 = leader3.wait_fragment(Duration::from_secs(2), check3)?;
        let status4 = leader4.wait_fragment(Duration::from_secs(2), check4)?;
        let status5 = leader5.wait_fragment(Duration::from_secs(2), check5)?;
        let status6 = leader6.wait_fragment(Duration::from_secs(2), check6)?;
        let status7 = leader7.wait_fragment(Duration::from_secs(2), check7)?;

        utils::assert_is_in_block(status1, &leader1)?;
        utils::assert_is_in_block(status2, &leader2)?;
        utils::assert_is_in_block(status3, &leader3)?;
        utils::assert_is_in_block(status4, &leader4)?;
        utils::assert_is_in_block(status5, &leader5)?;
        utils::assert_is_in_block(status6, &leader6)?;
        utils::assert_is_in_block(status7, &leader7)?;

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

    utils::assert_are_in_sync(
        SyncWaitParams::ZeroWait,
        vec![
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
    Ok(ScenarioResult::passed())
}
