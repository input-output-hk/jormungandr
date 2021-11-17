use crate::{
    node::{LeadershipMode, PersistenceMode},
    test::{utils, Result},
    Context, ScenarioResult,
};
use function_name::named;
const LEADER1: &str = "LEADER1";
const LEADER2: &str = "LEADER2";
const LEADER3: &str = "LEADER3";
const LEADER4: &str = "LEADER4";

#[named]
pub fn max_connections(context: Context) -> Result<ScenarioResult> {
    let name = function_name!();
    let scenario_settings = prepare_scenario! {
        name,
        &mut context,
        topology [
            LEADER1,
            LEADER2 -> LEADER1,
            LEADER3 -> LEADER1,
            LEADER4 -> LEADER1,
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
            ],
        }
    };

    let mut controller = scenario_settings.build(context)?;

    let mut leader1 = controller.spawn_node_custom(
        controller
            .new_spawn_params(LEADER1)
            .max_inbound_connections(2),
    )?;
    leader1.wait_for_bootstrap()?;

    let mut leader2 =
        controller.spawn_node(LEADER2, LeadershipMode::Leader, PersistenceMode::Persistent)?;
    leader2.wait_for_bootstrap()?;

    let mut leader3 =
        controller.spawn_node_custom(controller.new_spawn_params(LEADER3).max_connections(1))?;
    leader3.wait_for_bootstrap()?;

    let mut leader4 =
        controller.spawn_node(LEADER4, LeadershipMode::Leader, PersistenceMode::Persistent)?;
    leader4.wait_for_bootstrap()?;

    utils::wait(30);
    super::assert_connected_cnt(&leader1, 2, "leader1 should have only 2 nodes connected")?;

    leader1.shutdown()?;
    leader2.shutdown()?;
    leader3.shutdown()?;
    leader4.shutdown()?;
    controller.finalize();
    Ok(ScenarioResult::passed(name))
}
