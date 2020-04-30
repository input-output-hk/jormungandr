use crate::{
    node::{LeadershipMode, PersistenceMode},
    test::{utils, Result},
    Context, ScenarioResult,
};

use jormungandr_lib::{interfaces::Policy, time::Duration};

use rand_chacha::ChaChaRng;
use std::str::FromStr;
const LEADER1: &str = "LEADER1";
const LEADER2: &str = "LEADER2";
const LEADER3: &str = "LEADER3";

pub fn duplicated_node_id_test(mut context: Context<ChaChaRng>) -> Result<ScenarioResult> {
    let scenario_settings = prepare_scenario! {
        "duplicated_node_id_test",
        &mut context,
        topology [
            LEADER1,
            LEADER2 -> LEADER1,
            LEADER3 -> LEADER1
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
    let long_quarantine_policy = Policy {
        quarantine_duration: Some(Duration::new(30, 0)),
        quarantine_whitelist: None,
    };

    let leader1 = controller.spawn_node_custom(
        controller
            .new_spawn_params(LEADER1)
            .policy(long_quarantine_policy.clone()),
    )?;
    leader1.wait_for_bootstrap()?;

    let leader2 =
        controller.spawn_node(LEADER2, LeadershipMode::Leader, PersistenceMode::Persistent)?;
    leader2.wait_for_bootstrap()?;

    let leader2_node_id = leader2.stats()?.stats.expect("empty stats").node_id.clone();

    let mut leader3 =
        controller.spawn_node(LEADER3, LeadershipMode::Leader, PersistenceMode::Persistent)?;
    leader3.wait_for_bootstrap()?;

    utils::wait(10);

    let info_before = "before duplicated node id";
    super::assert_node_stats(&leader1, 2, 0, 2, 0, info_before)?;
    super::assert_are_in_network_stats(&leader1, vec![&leader2, &leader3], info_before)?;
    super::assert_are_available(&leader1, vec![&leader2, &leader3], info_before)?;
    super::assert_empty_quarantine(&leader1, info_before)?;
    super::assert_are_in_network_view(&leader1, vec![&leader2, &leader3], info_before)?;

    leader3.shutdown()?;
    leader3 = controller.spawn_node_custom(
        controller
            .new_spawn_params(LEADER3)
            .node_id(poldercast::Id::from_str(&leader2_node_id).unwrap()),
    )?;
    leader3.wait_for_bootstrap()?;

    utils::wait(30);

    leader2.log_stats();
    leader3.log_stats();
    leader1.log_stats();

    let info_after = "after leader3 duplicated node id";
    super::assert_node_stats(&leader1, 1, 1, 2, 0, &info_after)?;
    super::assert_are_in_network_stats(&leader1, vec![&&leader3], &info_after)?;
    super::assert_are_available(&leader1, vec![&leader3], &info_after)?;
    super::assert_are_in_quarantine(&leader1, vec![], &info_after)?;
    super::assert_are_in_network_view(&leader1, vec![&leader3], &info_after)?;

    leader1.shutdown()?;
    leader2.shutdown()?;
    leader3.shutdown()?;
    controller.finalize();
    Ok(ScenarioResult::passed())
}

pub fn duplicated_trusted_peer_id_test(mut context: Context<ChaChaRng>) -> Result<ScenarioResult> {
    let scenario_settings = prepare_scenario! {
        "duplicated_trusted_peer_id_test",
        &mut context,
        topology [
            LEADER1,
            LEADER2 -> LEADER1,
            LEADER3 -> LEADER1,
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

    let long_quarantine_policy = Policy {
        quarantine_duration: Some(Duration::new(30, 0)),
        quarantine_whitelist: None,
    };

    let mut controller = scenario_settings.build(context)?;

    let leader1 =
        controller.spawn_node(LEADER1, LeadershipMode::Leader, PersistenceMode::Persistent)?;
    leader1.wait_for_bootstrap()?;

    let leader2 = controller.spawn_node_custom(
        controller
            .new_spawn_params(LEADER2)
            .no_listen_address()
            .policy(long_quarantine_policy.clone()),
    )?;

    leader2.wait_for_bootstrap()?;

    utils::wait(10);

    let info_before = "before duplicated node id";
    super::assert_node_stats(&leader2, 1, 0, 1, 0, info_before)?;
    super::assert_are_in_network_stats(&leader2, vec![&leader1], info_before)?;
    super::assert_are_available(&leader2, vec![&leader1], info_before)?;
    super::assert_empty_quarantine(&leader2, info_before)?;
    super::assert_are_in_network_view(&leader2, vec![&leader1], info_before)?;

    let leader3 = controller.spawn_node_custom(
        controller
            .new_spawn_params(LEADER3)
            .no_listen_address()
            .node_id(leader1.public_id().clone()),
    )?;
    leader3.wait_for_bootstrap()?;

    utils::wait(30);

    let info_after = "after leader3 duplicated node id";
    super::assert_node_stats(&leader2, 1, 0, 1, 0, info_after)?;
    super::assert_are_in_network_stats(&leader2, vec![&leader1], info_after)?;
    super::assert_are_available(&leader2, vec![&leader1], info_after)?;
    super::assert_are_in_quarantine(&leader2, vec![], info_after)?;
    super::assert_are_in_network_view(&leader2, vec![&leader1], info_after)?;

    leader1.shutdown()?;
    leader2.shutdown()?;
    leader3.shutdown()?;
    controller.finalize();
    Ok(ScenarioResult::passed())
}
