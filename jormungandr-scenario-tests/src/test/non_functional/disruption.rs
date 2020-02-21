use crate::{
    node::{LeadershipMode, PersistenceMode},
    scenario::repository::ScenarioResult,
    test::utils::SyncWaitParams,
    test::{non_functional::*, utils, Result},
    Context,
};
use rand_chacha::ChaChaRng;

pub fn mesh_disruption(mut context: Context<ChaChaRng>) -> Result<ScenarioResult> {
    let scenario_settings = prepare_scenario! {
        "Disruption_Mesh",
        &mut context,
        topology [
            LEADER_1 -> LEADER_4,LEADER_5,
            LEADER_2 -> LEADER_1,LEADER_3,
            LEADER_3 -> LEADER_1,LEADER_4,
            LEADER_4 -> LEADER_5,
            LEADER_5 -> LEADER_3,LEADER_1,
        ]
        blockchain {
            consensus = GenesisPraos,
            number_of_slots_per_epoch = 60,
            slot_duration = 1,
            leaders = [ LEADER_1 ],
            initials = [
                account "unassigned1" with   500_000_000,
                account "delegated1" with  2_000_000_000 delegates to LEADER_3,
            ],
        }
    };

    let mut controller = scenario_settings.build(context)?;

    //monitor node disabled due to unsupported operation: restart node
    //controller.monitor_nodes();
    let mut leader5 = controller.spawn_node(
        LEADER_5,
        LeadershipMode::Leader,
        PersistenceMode::Persistent,
    )?;
    let leader4 = controller.spawn_node(
        LEADER_4,
        LeadershipMode::Leader,
        PersistenceMode::Persistent,
    )?;
    let leader3 = controller.spawn_node(
        LEADER_3,
        LeadershipMode::Leader,
        PersistenceMode::Persistent,
    )?;
    let mut leader2 = controller.spawn_node(
        LEADER_2,
        LeadershipMode::Leader,
        PersistenceMode::Persistent,
    )?;
    let leader1 = controller.spawn_node(
        LEADER_1,
        LeadershipMode::Leader,
        PersistenceMode::Persistent,
    )?;

    leader5.wait_for_bootstrap()?;
    leader4.wait_for_bootstrap()?;
    leader3.wait_for_bootstrap()?;
    leader2.wait_for_bootstrap()?;
    leader1.wait_for_bootstrap()?;

    let mut wallet1 = controller.wallet("unassigned1")?;
    let mut wallet2 = controller.wallet("delegated1")?;

    utils::sending_transactions_to_node_sequentially(
        10,
        &mut controller,
        &mut wallet1,
        &mut wallet2,
        &leader1,
    )?;

    leader2 =
        controller.restart_node(leader2, LeadershipMode::Leader, PersistenceMode::Persistent)?;

    utils::sending_transactions_to_node_sequentially(
        10,
        &mut controller,
        &mut wallet1,
        &mut wallet2,
        &leader5,
    )?;

    leader5 =
        controller.restart_node(leader5, LeadershipMode::Leader, PersistenceMode::Persistent)?;
    utils::sending_transactions_to_node_sequentially(
        10,
        &mut controller,
        &mut wallet1,
        &mut wallet2,
        &leader3,
    )?;

    utils::measure_and_log_sync_time(
        vec![&leader1, &leader2, &leader3, &leader4, &leader5],
        SyncWaitParams::nodes_restart(5).into(),
        "mesh_disruption_sync",
    );

    leader5.shutdown()?;
    leader4.shutdown()?;
    leader3.shutdown()?;
    leader2.shutdown()?;
    leader1.shutdown()?;
    Ok(ScenarioResult::passed())
}
