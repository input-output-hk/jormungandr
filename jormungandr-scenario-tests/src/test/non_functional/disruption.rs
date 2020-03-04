use crate::{
    node::{LeadershipMode, PersistenceMode},
    scenario::repository::ScenarioResult,
    test::utils::SyncWaitParams,
    test::{non_functional::*, utils, Result},
    Context,
};
use rand_chacha::ChaChaRng;

pub fn passive_leader_disruption_no_overlap(
    mut context: Context<ChaChaRng>,
) -> Result<ScenarioResult> {
    let scenario_settings = prepare_scenario! {
        "passive_leader_disruption_no_overlap",
        &mut context,
        topology [
            LEADER,
            PASSIVE -> LEADER,
        ]
        blockchain {
            consensus = GenesisPraos,
            number_of_slots_per_epoch = 60,
            slot_duration = 1,
            leaders = [ LEADER_1 ],
            initials = [
                account "unassigned1" with   500_000_000,
                account "delegated1" with  2_000_000_000 delegates to LEADER,
            ],
        }
    };

    let mut controller = scenario_settings.build(context)?;

    //monitor node disabled due to unsupported operation: restart node
    //controller.monitor_nodes();
    let leader =
        controller.spawn_node(LEADER, LeadershipMode::Leader, PersistenceMode::Persistent)?;
    let passive = controller.spawn_node(
        PASSIVE,
        LeadershipMode::Passive,
        PersistenceMode::Persistent,
    )?;

    leader.wait_for_bootstrap()?;
    passive.wait_for_bootstrap()?;

    // 1. both nodes are up
    utils::wait(5);

    // 2. Only passive is down
    leader.shutdown()?;

    // 3. No node is up
    passive.shutdown()?;

    // 4. Only leader is up
    let leader =
        controller.spawn_node(LEADER, LeadershipMode::Leader, PersistenceMode::Persistent)?;
    leader.wait_for_bootstrap()?;
    utils::wait(5);

    // 5. No node is up
    leader.shutdown()?;

    //6. Both nodes are up
    let leader =
        controller.spawn_node(LEADER, LeadershipMode::Leader, PersistenceMode::Persistent)?;
    let passive = controller.spawn_node(
        PASSIVE,
        LeadershipMode::Passive,
        PersistenceMode::Persistent,
    )?;

    leader.wait_for_bootstrap()?;
    passive.wait_for_bootstrap()?;

    utils::measure_and_log_sync_time(
        vec![&leader, &passive],
        SyncWaitParams::nodes_restart(5).into(),
        "passive_leader_disruption_no_overlap",
    );

    leader.shutdown()?;
    passive.shutdown()?;
    controller.finalize();
    Ok(ScenarioResult::passed())
}

pub fn passive_leader_disruption_overlap(
    mut context: Context<ChaChaRng>,
) -> Result<ScenarioResult> {
    let scenario_settings = prepare_scenario! {
        "passive_leader_disruption_overlap",
        &mut context,
        topology [
            LEADER,
            PASSIVE -> LEADER,
        ]
        blockchain {
            consensus = GenesisPraos,
            number_of_slots_per_epoch = 60,
            slot_duration = 1,
            leaders = [ LEADER_1 ],
            initials = [
                account "unassigned1" with   500_000_000,
                account "delegated1" with  2_000_000_000 delegates to LEADER,
            ],
        }
    };

    let mut controller = scenario_settings.build(context)?;

    //monitor node disabled due to unsupported operation: restart node
    //controller.monitor_nodes();
    let leader =
        controller.spawn_node(LEADER, LeadershipMode::Leader, PersistenceMode::Persistent)?;
    let passive = controller.spawn_node(
        PASSIVE,
        LeadershipMode::Passive,
        PersistenceMode::Persistent,
    )?;

    leader.wait_for_bootstrap()?;
    passive.wait_for_bootstrap()?;

    // 1. both nodes are up
    utils::wait(5);

    // 2. Only leader is up
    passive.shutdown()?;

    // 3. Both nodes are up
    let passive = controller.spawn_node(
        PASSIVE,
        LeadershipMode::Passive,
        PersistenceMode::Persistent,
    )?;
    passive.wait_for_bootstrap()?;

    utils::measure_and_log_sync_time(
        vec![&leader, &passive],
        SyncWaitParams::nodes_restart(5).into(),
        "passive_leader_disruption_overlap",
    );

    leader.shutdown()?;
    passive.shutdown()?;
    controller.finalize();
    Ok(ScenarioResult::passed())
}

pub fn leader_leader_disruption_overlap(mut context: Context<ChaChaRng>) -> Result<ScenarioResult> {
    let scenario_settings = prepare_scenario! {
        "leader_leader_disruption_overlap",
        &mut context,
        topology [
            LEADER_2,
            LEADER_1 -> LEADER_2,
        ]
        blockchain {
            consensus = GenesisPraos,
            number_of_slots_per_epoch = 60,
            slot_duration = 1,
            leaders = [ LEADER_1 ],
            initials = [
                account "unassigned1" with   500_000_000,
                account "delegated1" with  2_000_000_000 delegates to LEADER_2,
            ],
        }
    };

    let mut controller = scenario_settings.build(context)?;

    //monitor node disabled due to unsupported operation: restart node
    //controller.monitor_nodes();

    let leader2 = controller.spawn_node(
        LEADER_2,
        LeadershipMode::Leader,
        PersistenceMode::Persistent,
    )?;
    leader2.wait_for_bootstrap()?;

    // 1. second node is up
    utils::wait(5);

    // 2. Both nodes are up
    let leader1 = controller.spawn_node(
        LEADER_1,
        LeadershipMode::Leader,
        PersistenceMode::Persistent,
    )?;
    leader1.wait_for_bootstrap()?;
    utils::wait(5);

    // 3. second node is down
    leader2.shutdown()?;
    utils::wait(5);

    // 4. both nodes are up
    let leader2 = controller.spawn_node(
        LEADER_2,
        LeadershipMode::Leader,
        PersistenceMode::Persistent,
    )?;
    leader2.wait_for_bootstrap()?;

    utils::measure_and_log_sync_time(
        vec![&leader1, &leader2],
        SyncWaitParams::nodes_restart(5).into(),
        "leader_leader_disruption_overlap",
    );

    leader1.shutdown()?;
    leader2.shutdown()?;
    controller.finalize();
    Ok(ScenarioResult::passed())
}

pub fn leader_leader_disruption_no_overlap(
    mut context: Context<ChaChaRng>,
) -> Result<ScenarioResult> {
    let scenario_settings = prepare_scenario! {
        "leader_leader_disruption_no_overlap",
        &mut context,
        topology [
            LEADER_2,
            LEADER_1 -> LEADER_2,
        ]
        blockchain {
            consensus = GenesisPraos,
            number_of_slots_per_epoch = 60,
            slot_duration = 1,
            leaders = [ LEADER_1 ],
            initials = [
                account "unassigned1" with   500_000_000,
                account "delegated1" with  2_000_000_000 delegates to LEADER_2,
            ],
        }
    };

    let mut controller = scenario_settings.build(context)?;

    //monitor node disabled due to unsupported operation: restart node
    //controller.monitor_nodes();

    let leader2 = controller.spawn_node(
        LEADER_2,
        LeadershipMode::Leader,
        PersistenceMode::Persistent,
    )?;

    let leader1 = controller.spawn_node(
        LEADER_1,
        LeadershipMode::Leader,
        PersistenceMode::Persistent,
    )?;
    leader2.wait_for_bootstrap()?;
    leader1.wait_for_bootstrap()?;

    // 1. Both nodes are up
    utils::wait(5);

    // 2. Only node 2 is up
    leader1.shutdown()?;

    // 3. No nodes are up
    leader2.shutdown()?;

    // 4. Only node 1 is up
    let leader1 = controller.spawn_node(
        LEADER_1,
        LeadershipMode::Leader,
        PersistenceMode::Persistent,
    )?;
    leader1.wait_for_bootstrap()?;

    // 5. No nodes are up
    leader1.shutdown()?;

    // 6. Both nodes are up

    let leader2 = controller.spawn_node(
        LEADER_2,
        LeadershipMode::Leader,
        PersistenceMode::Persistent,
    )?;

    let leader1 = controller.spawn_node(
        LEADER_1,
        LeadershipMode::Leader,
        PersistenceMode::Persistent,
    )?;
    leader2.wait_for_bootstrap()?;
    leader1.wait_for_bootstrap()?;

    utils::measure_and_log_sync_time(
        vec![&leader1, &leader2],
        SyncWaitParams::nodes_restart(5).into(),
        "leader_leader_disruption_no_overlap",
    );

    leader1.shutdown()?;
    leader2.shutdown()?;
    controller.finalize();
    Ok(ScenarioResult::passed())
}

pub fn point_to_point_disruption(mut context: Context<ChaChaRng>) -> Result<ScenarioResult> {
    let scenario_settings = prepare_scenario! {
        "point_to_point_disruption",
        &mut context,
        topology [
            LEADER_2,
            LEADER_1 -> LEADER_2,
            LEADER_3 -> LEADER_2
        ]
        blockchain {
            consensus = GenesisPraos,
            number_of_slots_per_epoch = 60,
            slot_duration = 1,
            leaders = [ LEADER_1 ],
            initials = [
                account "unassigned1" with   500_000_000,
                account "delegated1" with  2_000_000_000 delegates to LEADER_2,
            ],
        }
    };

    let mut controller = scenario_settings.build(context)?;

    let mut wallet1 = controller.wallet("unassigned1")?;
    let mut wallet2 = controller.wallet("delegated1")?;

    //monitor node disabled due to unsupported operation: restart node
    //controller.monitor_nodes();
    let leader2 = controller.spawn_node(
        LEADER_2,
        LeadershipMode::Leader,
        PersistenceMode::Persistent,
    )?;
    let leader1 = controller.spawn_node(
        LEADER_1,
        LeadershipMode::Leader,
        PersistenceMode::Persistent,
    )?;
    let leader3 = controller.spawn_node(
        LEADER_3,
        LeadershipMode::Leader,
        PersistenceMode::Persistent,
    )?;

    leader2.wait_for_bootstrap()?;
    leader1.wait_for_bootstrap()?;
    leader3.wait_for_bootstrap()?;

    utils::sending_transactions_to_node_sequentially(
        10,
        &mut controller,
        &mut wallet1,
        &mut wallet2,
        &leader1,
    )?;

    leader2.shutdown()?;

    utils::measure_and_log_sync_time(
        vec![&leader1, &leader3],
        SyncWaitParams::nodes_restart(5).into(),
        "point_to_point_disruption",
    );

    leader3.shutdown()?;
    leader1.shutdown()?;
    controller.finalize();
    Ok(ScenarioResult::passed())
}

pub fn point_to_point_disruption_overlap(
    mut context: Context<ChaChaRng>,
) -> Result<ScenarioResult> {
    let scenario_settings = prepare_scenario! {
        "point_to_point_disruption_overlap",
        &mut context,
        topology [
            LEADER_2,
            LEADER_1 -> LEADER_2,
            LEADER_3 -> LEADER_2
        ]
        blockchain {
            consensus = GenesisPraos,
            number_of_slots_per_epoch = 60,
            slot_duration = 1,
            leaders = [ LEADER_1 ],
            initials = [
                account "unassigned1" with   500_000_000,
                account "delegated1" with  2_000_000_000 delegates to LEADER_2,
            ],
        }
    };

    let mut controller = scenario_settings.build(context)?;

    //monitor node disabled due to unsupported operation: restart node
    //controller.monitor_nodes();
    let mut leader2 = controller.spawn_node(
        LEADER_2,
        LeadershipMode::Leader,
        PersistenceMode::Persistent,
    )?;
    let mut leader1 = controller.spawn_node(
        LEADER_1,
        LeadershipMode::Leader,
        PersistenceMode::Persistent,
    )?;

    leader2.wait_for_bootstrap()?;
    leader1.wait_for_bootstrap()?;

    //1. 2 and 1 is up
    utils::wait(5);

    //2. node 1 is down
    leader1.shutdown()?;

    let mut leader3 = controller.spawn_node(
        LEADER_3,
        LeadershipMode::Leader,
        PersistenceMode::Persistent,
    )?;
    leader3.wait_for_bootstrap()?;

    //3. only Node 3 is up
    leader2.shutdown()?;

    //4. 1 and 3 is up
    leader1 = controller.spawn_node(
        LEADER_1,
        LeadershipMode::Leader,
        PersistenceMode::Persistent,
    )?;
    leader1.wait_for_bootstrap()?;

    leader3 = controller.spawn_node(
        LEADER_3,
        LeadershipMode::Leader,
        PersistenceMode::Persistent,
    )?;
    leader3.wait_for_bootstrap()?;

    //5. 2 and 3 is up
    leader1.shutdown()?;
    leader2 = controller.spawn_node(
        LEADER_2,
        LeadershipMode::Leader,
        PersistenceMode::Persistent,
    )?;
    leader2.wait_for_bootstrap()?;

    //6. 1 and 2 is up
    leader3.shutdown()?;

    leader1 = controller.spawn_node(
        LEADER_1,
        LeadershipMode::Leader,
        PersistenceMode::Persistent,
    )?;
    leader1.wait_for_bootstrap()?;

    //7. only Node 3 is up
    leader1.shutdown()?;
    leader2.shutdown()?;

    leader3 = controller.spawn_node(
        LEADER_3,
        LeadershipMode::Leader,
        PersistenceMode::Persistent,
    )?;
    leader3.wait_for_bootstrap()?;

    //8. 1 and 3 is up
    leader1 = controller.spawn_node(
        LEADER_1,
        LeadershipMode::Leader,
        PersistenceMode::Persistent,
    )?;
    leader1.wait_for_bootstrap()?;

    //9. all nodes are up
    leader2 = controller.spawn_node(
        LEADER_2,
        LeadershipMode::Leader,
        PersistenceMode::Persistent,
    )?;
    leader2.wait_for_bootstrap()?;

    utils::measure_and_log_sync_time(
        vec![&leader1, &leader2, &leader3],
        SyncWaitParams::nodes_restart(5).into(),
        "point_to_point_disruption_overlap",
    );

    leader3.shutdown()?;
    leader2.shutdown()?;
    leader1.shutdown()?;
    controller.finalize();
    Ok(ScenarioResult::passed())
}

pub fn custom_network_disruption(mut context: Context<ChaChaRng>) -> Result<ScenarioResult> {
    let scenario_settings = prepare_scenario! {
        "custom_network_disruption",
        &mut context,
        topology [
            LEADER_5,
            LEADER_1 -> LEADER_3,
            LEADER_2 -> LEADER_3,LEADER_5,
            LEADER_3 -> LEADER_5,
            LEADER_4 -> LEADER_5,
            PASSIVE -> LEADER_5,
        ]
        blockchain {
            consensus = GenesisPraos,
            number_of_slots_per_epoch = 60,
            slot_duration = 1,
            leaders = [ LEADER_1 ],
            initials = [
                account "unassigned1" with   500_000_000,
                account "delegated1" with  2_000_000_000 delegates to LEADER_1,
                account "delegated3" with  2_000_000_000 delegates to LEADER_3,
                account "delegated4" with  2_000_000_000 delegates to LEADER_4,
            ],
        }
    };

    let mut controller = scenario_settings.build(context)?;

    //monitor node disabled due to unsupported operation: restart node
    //controller.monitor_nodes();
    let leader5 = controller.spawn_node(
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
    let leader2 = controller.spawn_node(
        LEADER_2,
        LeadershipMode::Leader,
        PersistenceMode::Persistent,
    )?;

    leader5.wait_for_bootstrap()?;
    leader4.wait_for_bootstrap()?;
    leader3.wait_for_bootstrap()?;
    leader2.wait_for_bootstrap()?;

    let mut wallet1 = controller.wallet("delegated1")?;
    let mut wallet3 = controller.wallet("delegated3")?;

    utils::sending_transactions_to_node_sequentially(
        2,
        &mut controller,
        &mut wallet1,
        &mut wallet3,
        &leader2,
    )?;

    let leader1 = controller.spawn_node(
        LEADER_1,
        LeadershipMode::Leader,
        PersistenceMode::Persistent,
    )?;
    leader1.wait_for_bootstrap()?;

    utils::sending_transactions_to_node_sequentially(
        2,
        &mut controller,
        &mut wallet1,
        &mut wallet3,
        &leader1,
    )?;

    leader2.shutdown()?;

    let passive = controller.spawn_node(
        PASSIVE,
        LeadershipMode::Passive,
        PersistenceMode::Persistent,
    )?;
    passive.wait_for_bootstrap()?;

    utils::sending_transactions_to_node_sequentially(
        2,
        &mut controller,
        &mut wallet1,
        &mut wallet3,
        &passive,
    )?;

    leader5.shutdown()?;

    utils::measure_and_log_sync_time(
        vec![&leader1, &leader2, &leader3, &leader4, &leader5, &passive],
        SyncWaitParams::nodes_restart(5).into(),
        "custom_network_disruption",
    );

    passive.shutdown()?;
    leader4.shutdown()?;
    leader3.shutdown()?;
    leader2.shutdown()?;
    leader1.shutdown()?;
    controller.finalize();
    Ok(ScenarioResult::passed())
}

pub fn mesh_disruption(mut context: Context<ChaChaRng>) -> Result<ScenarioResult> {
    let scenario_settings = prepare_scenario! {
        "Disruption_Mesh",
        &mut context,
        topology [
            LEADER_4,
            LEADER_1 -> LEADER_4,LEADER_5,
            LEADER_2 -> LEADER_1,LEADER_3,
            LEADER_3 -> LEADER_1,LEADER_4,
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
        &leader1,
    )?;

    leader5 =
        controller.restart_node(leader5, LeadershipMode::Leader, PersistenceMode::Persistent)?;
    utils::sending_transactions_to_node_sequentially(
        10,
        &mut controller,
        &mut wallet1,
        &mut wallet2,
        &leader1,
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
    controller.finalize();
    Ok(ScenarioResult::passed())
}
