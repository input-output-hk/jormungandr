use super::{LEADER, PASSIVE};
use crate::scenario::{repository::ScenarioResult, Context, Controller};
use crate::test::Result;
use function_name::named;
use jormungandr_testing_utils::{
    stake_pool::StakePool,
    testing::{
        network::{LeadershipMode, PersistenceMode},
        node::{download_last_n_releases, get_jormungandr_bin},
        FragmentNode, SyncNode,
    },
    version_0_8_19, Version,
};
use rand_chacha::ChaChaRng;
use std::path::PathBuf;

#[named]
pub fn legacy_current_node_fragment_propagation(
    mut context: Context<ChaChaRng>,
) -> Result<ScenarioResult> {
    let title = "test_legacy_current_node_fragment_propagation";
    let scenario_settings = prepare_scenario! {
        title,
        &mut context,
        topology [
            LEADER,
            PASSIVE -> LEADER,
        ]
        blockchain {
            consensus = GenesisPraos,
            number_of_slots_per_epoch = 60,
            slot_duration = 1,
            leaders = [ LEADER],
            initials = [
                "account" "alice" with  2_000_000_000 delegates to LEADER,
                "account" "bob" with   500_000_000,
                "account" "clarice" with   500_000_000,
                "account" "david" with   500_000_000,
            ],
        }
    };

    let (legacy_app, version) = get_legacy_data(title, &mut context);
    let mut controller = scenario_settings.build(context)?;
    controller.monitor_nodes();

    let leader =
        controller.spawn_node(LEADER, LeadershipMode::Leader, PersistenceMode::InMemory)?;
    leader.wait_for_bootstrap()?;

    let passive = controller.spawn_legacy_node(
        controller
            .new_spawn_params(PASSIVE)
            .leadership_mode(LeadershipMode::Passive)
            .persistence_mode(PersistenceMode::InMemory)
            .jormungandr(legacy_app),
        &version,
    )?;
    passive.wait_for_bootstrap()?;

    send_all_fragment_types(&mut controller, &passive, Some(version));

    leader.shutdown()?;
    passive.shutdown()?;

    controller.finalize();
    Ok(ScenarioResult::passed(title))
}

#[named]
pub fn current_node_legacy_fragment_propagation(
    mut context: Context<ChaChaRng>,
) -> Result<ScenarioResult> {
    let title = "test_legacy_current_node_fragment_propagation";
    let scenario_settings = prepare_scenario! {
        title,
        &mut context,
        topology [
            LEADER,
            PASSIVE -> LEADER,
        ]
        blockchain {
            consensus = GenesisPraos,
            number_of_slots_per_epoch = 60,
            slot_duration = 1,
            leaders = [ LEADER],
            initials = [
                "account" "alice" with  2_000_000_000 delegates to LEADER,
                "account" "bob" with   500_000_000,
                "account" "clarice" with   500_000_000,
                "account" "david" with   500_000_000,
            ],
        }
    };

    let (legacy_app, version) = get_legacy_data(title, &mut context);

    let mut controller = scenario_settings.build(context)?;
    controller.monitor_nodes();

    let leader = controller.spawn_legacy_node(
        controller
            .new_spawn_params(LEADER)
            .leadership_mode(LeadershipMode::Leader)
            .persistence_mode(PersistenceMode::InMemory)
            .jormungandr(legacy_app),
        &version,
    )?;
    leader.wait_for_bootstrap()?;

    let passive =
        controller.spawn_node(PASSIVE, LeadershipMode::Passive, PersistenceMode::InMemory)?;
    passive.wait_for_bootstrap()?;

    send_all_fragment_types(&mut controller, &passive, Some(version));

    leader.shutdown()?;
    passive.shutdown()?;

    controller.finalize();
    Ok(ScenarioResult::passed(title))
}

#[named]
pub fn current_node_fragment_propagation(
    mut context: Context<ChaChaRng>,
) -> Result<ScenarioResult> {
    let title = "test_legacy_current_node_fragment_propagation";
    let scenario_settings = prepare_scenario! {
        title,
        &mut context,
        topology [
            LEADER,
            PASSIVE -> LEADER,
        ]
        blockchain {
            consensus = GenesisPraos,
            number_of_slots_per_epoch = 60,
            slot_duration = 1,
            leaders = [ LEADER],
            initials = [
                "account" "alice" with  2_000_000_000 delegates to LEADER,
                "account" "bob" with   500_000_000,
                "account" "clarice" with   500_000_000,
                "account" "david" with   500_000_000,
            ],
        }
    };

    let mut controller = scenario_settings.build(context)?;
    controller.monitor_nodes();

    let leader =
        controller.spawn_node(LEADER, LeadershipMode::Leader, PersistenceMode::InMemory)?;
    leader.wait_for_bootstrap()?;

    let passive =
        controller.spawn_node(PASSIVE, LeadershipMode::Passive, PersistenceMode::InMemory)?;
    passive.wait_for_bootstrap()?;

    send_all_fragment_types(&mut controller, &passive, None);

    leader.shutdown()?;
    passive.shutdown()?;

    controller.finalize();
    Ok(ScenarioResult::passed(title))
}

fn get_legacy_data(title: &str, context: &mut Context<ChaChaRng>) -> (PathBuf, Version) {
    let releases = download_last_n_releases(1);
    let last_release = releases.last().unwrap();
    let legacy_app = get_jormungandr_bin(last_release, &context.child_directory(title));
    (legacy_app, last_release.version())
}

fn send_all_fragment_types<A: FragmentNode + SyncNode + Sized + Send>(
    controller: &mut Controller,
    passive: &A,
    version: Option<Version>,
) {
    let mut alice = controller.wallet("alice").unwrap();
    let mut bob = controller.wallet("bob").unwrap();
    let clarice = controller.wallet("clarice").unwrap();
    let mut david = controller.wallet("david").unwrap();

    let leader_stake_pool = controller.stake_pool(LEADER).unwrap();
    let david_stake_pool = StakePool::new(&david);

    let sender = controller.fragment_sender();

    sender
        .send_transaction(&mut alice, &bob, passive, 10.into())
        .expect("send transaction failed");
    sender
        .send_pool_registration(&mut david, &david_stake_pool, passive)
        .expect("send pool registration");
    sender
        .send_owner_delegation(&mut david, &david_stake_pool, passive)
        .expect("send owner delegation");
    sender
        .send_full_delegation(&mut bob, &leader_stake_pool, passive)
        .expect("send full delegation failed");

    let distribution: Vec<(&StakePool, u8)> = vec![(&leader_stake_pool, 1), (&david_stake_pool, 1)];
    sender
        .send_split_delegation(&mut bob, &distribution, passive)
        .expect("send split delegation failed");

    let mut david_and_clarice_stake_pool = david_stake_pool.clone();
    david_and_clarice_stake_pool
        .info_mut()
        .owners
        .push(clarice.identifier().into_public_key());

    if let Some(version) = version {
        if version != version_0_8_19() {
            sender
                .send_pool_update(
                    &mut david,
                    &david_stake_pool,
                    &david_and_clarice_stake_pool,
                    passive,
                )
                .expect("send update stake pool failed");
        }
    }

    sender
        .send_pool_retire(&mut david, &david_stake_pool, passive)
        .expect("send pool retire failed");
}
