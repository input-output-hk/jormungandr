use crate::{
    node::{LeadershipMode, PersistenceMode},
    test::{utils, Result},
    Context, ScenarioResult,
};
use rand_chacha::ChaChaRng;
const LEADER1: &str = "LEADER1";
const LEADER2: &str = "LEADER2";
const LEADER3: &str = "LEADER3";
const LEADER4: &str = "LEADER4";

use function_name::named;

#[named]
pub fn p2p_stats_test(context: Context<ChaChaRng>) -> Result<ScenarioResult> {
    let name = function_name!();
    let scenario_settings = prepare_scenario! {
        name,
        &mut context,
        topology [
            LEADER1,
            LEADER2 -> LEADER1,
            LEADER3 -> LEADER1,
            LEADER4 -> LEADER2 -> LEADER3,
        ]
        blockchain {
            consensus = GenesisPraos,
            number_of_slots_per_epoch = 60,
            slot_duration = 2,
            leaders = [ LEADER1 ],
            initials = [
                "account" "delegated1" with  2_000_000_000 delegates to LEADER1,
                "account" "delegated2" with  2_000_000_000 delegates to LEADER2,
                "account" "delegated3" with  2_000_000_000 delegates to LEADER3,
                "account" "delegated4" with  2_000_000_000 delegates to LEADER4,
            ],
        }
    };

    let mut controller = scenario_settings.build(context)?;

    let mut leader1 =
        controller.spawn_node(LEADER1, LeadershipMode::Leader, PersistenceMode::Persistent)?;
    leader1.wait_for_bootstrap()?;

    super::assert_node_stats(&leader1, 0, 0, 0, "no peers for leader1")?;
    let info_before = "no peers for leader 1";
    utils::assert_equals(
        &vec![],
        &leader1.rest().network_stats()?,
        &format!("{} network_stats", info_before),
    )?;
    utils::assert_equals(
        &vec![],
        &leader1.rest().p2p_quarantined()?,
        &format!("{} p2p_quarantined", info_before),
    )?;
    utils::assert_equals(
        &vec![],
        &leader1.rest().p2p_non_public()?,
        &format!("{} p2p_non_public", info_before),
    )?;
    utils::assert_equals(
        &vec![],
        &leader1.rest().p2p_available()?,
        &format!("{} p2p_available", info_before),
    )?;
    utils::assert_equals(
        &vec![],
        &leader1.rest().p2p_view()?,
        &format!("{} p2p_view", info_before),
    )?;

    let mut leader2 =
        controller.spawn_node_custom(controller.new_spawn_params(LEADER2).no_listen_address())?;
    leader2.wait_for_bootstrap()?;
    utils::wait(20);
    super::assert_node_stats(&leader1, 1, 0, 1, "bootstrapped leader1")?;
    super::assert_node_stats(&leader2, 1, 0, 1, "bootstrapped leader2")?;

    let mut leader3 =
        controller.spawn_node_custom(controller.new_spawn_params(LEADER3).no_listen_address())?;

    leader3.wait_for_bootstrap()?;
    utils::wait(20);
    super::assert_node_stats(&leader1, 2, 0, 2, "leader1: leader3 node is up")?;
    super::assert_node_stats(&leader2, 2, 0, 2, "leader2: leader3 node is up")?;
    super::assert_node_stats(&leader3, 2, 0, 2, "leader3: leader3 node is up")?;

    let mut leader4 =
        controller.spawn_node_custom(controller.new_spawn_params(LEADER4).no_listen_address())?;

    leader4.wait_for_bootstrap()?;
    utils::wait(20);
    super::assert_node_stats(&leader1, 3, 0, 3, "leader1: leader4 node is up")?;
    super::assert_node_stats(&leader2, 3, 0, 3, "leader2: leader4 node is up")?;
    super::assert_node_stats(&leader3, 3, 0, 3, "leader3: leader4 node is up")?;
    super::assert_node_stats(&leader3, 3, 0, 3, "leader4: leader4 node is up")?;

    leader2.shutdown()?;
    utils::wait(20);
    super::assert_node_stats(&leader1, 2, 1, 3, "leader1: leader 2 is down")?;
    super::assert_node_stats(&leader3, 2, 1, 3, "leader3: leader 2 is down")?;
    super::assert_node_stats(&leader4, 2, 1, 3, "leader4: leader 2 is down")?;

    leader1.shutdown()?;
    leader3.shutdown()?;
    leader4.shutdown()?;
    controller.finalize();
    Ok(ScenarioResult::passed(name))
}
