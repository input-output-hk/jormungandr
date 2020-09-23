use crate::{
    node::NodeController,
    node::{LeadershipMode, PersistenceMode},
    test::{utils, Result},
    Context, ScenarioResult,
};

use std::time::{Duration, SystemTime};

use jortestkit::process::sleep;
use rand_chacha::ChaChaRng;
const LEADER_1: &str = "Leader1";
const LEADER_2: &str = "Leader2";

pub fn leader_restart_preserves_leadership_log(
    mut context: Context<ChaChaRng>,
) -> Result<ScenarioResult> {
    let scenario_settings = prepare_scenario! {
        "Passive node promotion to leader",
        &mut context,
        topology [
            LEADER_1,
            LEADER_2 -> LEADER_1,
        ]
        blockchain {
            consensus = Bft,
            number_of_slots_per_epoch = 120,
            slot_duration = 2,
            leaders = [ LEADER_1, LEADER_2 ],
            initials = [
                account "alice" with   500_000_000,
                account "bob" with   500_000_000,
            ],
        }
    };

    let mut controller = scenario_settings.build(context)?;

    let now = SystemTime::now();

    let mut leader_1 = controller.spawn_node(
        LEADER_1,
        LeadershipMode::Leader,
        PersistenceMode::Persistent,
    )?;
    leader_1.wait_for_bootstrap()?;

    controller.monitor_nodes();

    //wait more than half an epoch
    while now.elapsed().unwrap().as_secs() < 200 {
        sleep(1);
    }

    //start bft node 2
    let leader_2 = controller.spawn_node(
        LEADER_2,
        LeadershipMode::Leader,
        PersistenceMode::Persistent,
    )?;
    leader_2.wait_for_bootstrap()?;

    // logs during epoch 0 should be empty
    utils::assert_equals(
        &leader_2.leadership_log()?.len(),
        &0,
        "leadeship log should be empty",
    )?;

    while now.elapsed().unwrap().as_secs() < 250 {
        sleep(1);
    }

    // logs during epoch 0 should be empty
    utils::assert_equals(
        &(leader_2.leadership_log()?.len() > 0),
        &true,
        "leadeship log should NOT be empty",
    )?;

    leader_2.shutdown()?;
    leader_1.shutdown()?;
    controller.finalize();
    Ok(ScenarioResult::passed())
}
