use crate::{
    node::NodeController,
    node::{LeadershipMode, PersistenceMode},
    test::{utils, Result},
    Context, ScenarioResult,
};
use jormungandr_lib::interfaces::EnclaveLeaderId;
use rand_chacha::ChaChaRng;
const LEADER: &str = "Leader";
const PASSIVE: &str = "Passive";

pub fn passive_node_promotion(mut context: Context<ChaChaRng>) -> Result<ScenarioResult> {
    let scenario_settings = prepare_scenario! {
        "Passive node promotion to leader",
        &mut context,
        topology [
            LEADER,
            PASSIVE -> LEADER,
        ]
        blockchain {
            consensus = GenesisPraos,
            number_of_slots_per_epoch = 60,
            slot_duration = 1,
            leaders = [ LEADER ],
            initials = [
                account "unassigned1" with   500_000_000,
                account "delegated1" with  2_000_000_000 delegates to LEADER,
                account "delegated2" with  2_000_000_000 delegates to PASSIVE,
            ],
        }
    };

    let mut controller = scenario_settings.build(context)?;

    let leader =
        controller.spawn_node(LEADER, LeadershipMode::Leader, PersistenceMode::InMemory)?;
    leader.wait_for_bootstrap()?;
    let passive =
        controller.spawn_node(PASSIVE, LeadershipMode::Passive, PersistenceMode::InMemory)?;
    passive.wait_for_bootstrap()?;
    controller.monitor_nodes();

    // promote leader
    promote_and_assert_leaders_id(&passive, 1.into(), 1, "after promotion")?;

    // demote leader
    demote_and_assert_leaders_id(&passive, 1, 0, "after demotion")?;

    // promote leader again
    promote_and_assert_leaders_id(&passive, 1.into(), 1, "second promotion")?;

    // promote duplicated leader
    promote_and_assert_leaders_id(&passive, 1.into(), 1, "duplicated promotion")?;

    passive.shutdown()?;
    leader.shutdown()?;
    controller.finalize();
    Ok(ScenarioResult::passed())
}

fn promote_and_assert_leaders_id(
    node: &NodeController,
    expected_leader_id: EnclaveLeaderId,
    expected_length: u32,
    info: &str,
) -> Result<()> {
    let leader_id = node.promote()?;
    let leaders_length: u32 = node.leaders()?.len() as u32;

    utils::assert_equals(
        &leader_id,
        &expected_leader_id,
        &format!("leader id {}", info),
    )?;
    utils::assert_equals(
        &leaders_length,
        &expected_length,
        &format!("leaders count {}", info),
    )?;
    Ok(())
}

fn demote_and_assert_leaders_id(
    node: &NodeController,
    leader_id: u32,
    expected_length: u32,
    info: &str,
) -> Result<()> {
    node.demote(leader_id)?;
    let actual_length: u32 = node.leaders()?.len() as u32;

    utils::assert_equals(
        &actual_length,
        &expected_length,
        &format!("leaders count {}", info),
    )?;
    Ok(())
}
