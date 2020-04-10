use crate::{
    node::NodeController,
    node::{LeadershipMode, PersistenceMode},
    test::{utils, Result},
    Context, ScenarioResult,
};
use rand_chacha::ChaChaRng;
const LEADER1: &str = "LEADER1";
const LEADER2: &str = "LEADER2";
const LEADER3: &str = "LEADER3";
const LEADER4: &str = "LEADER4";

pub fn max_connections(mut context: Context<ChaChaRng>) -> Result<ScenarioResult> {
    let scenario_settings = prepare_scenario! {
        "p2p stats",
        &mut context,
        topology [
            LEADER1,
            LEADER2 -> LEADER1,
            LEADER3 -> LEADER2 -> LEADER1,
            LEADER4 -> LEADER2 -> LEADER1,
        ]
        blockchain {
            consensus = GenesisPraos,
            number_of_slots_per_epoch = 60,
            slot_duration = 2,
            leaders = [ LEADER1 ],
            initials = [
                account "delegated1" with  2_000_000_000 delegates to LEADER1,
                account "delegated2" with  2_000_000_000 delegates to LEADER2,
                account "delegated3" with  2_000_000_000 delegates to LEADER3,
            ],
        }
    };

    let mut controller = scenario_settings.build(context)?;

    let leader1 = controller.spawn_node_custom(
        controller
            .new_spawn_params(LEADER1)
            .max_inbound_connections(2),
    )?;
    leader1.wait_for_bootstrap()?;

    let leader2 =
        controller.spawn_node(LEADER2, LeadershipMode::Leader, PersistenceMode::Persistent)?;
    leader2.wait_for_bootstrap()?;

    let leader3 =
        controller.spawn_node_custom(controller.new_spawn_params(LEADER3).max_connections(1))?;
    leader3.wait_for_bootstrap()?;

    utils::wait(10);

    super::assert_connected_cnt(
        &leader3,
        1,
        "leader3 should be connected to 1 node (leader1)",
    )?;
    super::assert_are_in_network_view(
        &leader3,
        vec![&leader2, &leader1],
        "leader3 should have leader2 in network view only",
    )?;

    let leader4 =
        controller.spawn_node(LEADER4, LeadershipMode::Leader, PersistenceMode::Persistent)?;
    leader4.wait_for_bootstrap()?;

    utils::wait(30);
    super::assert_connected_cnt(
        &leader4,
        2,
        "leader4 should only connect to 2 nodes (leader2,leader3)",
    )?;
    super::assert_connected_cnt(&leader3, 1, "leader3 should be connected to 1 node")?;
    super::assert_connected_cnt(&leader2, 1, "leader2 should be connected to 1 node leader1")?;
    super::assert_are_in_network_view(
        &leader4,
        vec![&leader2, &leader1],
        "leader4 should have only 2 nodes in network view",
    )?;

    leader1.shutdown()?;
    leader2.shutdown()?;
    leader3.shutdown()?;
    leader4.shutdown()?;
    controller.finalize();
    Ok(ScenarioResult::passed())
}
