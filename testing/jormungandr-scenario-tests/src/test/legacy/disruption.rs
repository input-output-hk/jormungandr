use crate::{
    test::{
        utils::{self, MeasurementReportInterval, SyncNode, SyncWaitParams},
        Result,
    },
    Context, ScenarioResult,
};

use jormungandr_testing_utils::testing::network::{LeadershipMode, PersistenceMode};
use jormungandr_testing_utils::testing::FragmentSender;
use jormungandr_testing_utils::{
    testing::{
        node::{download_last_n_releases, get_jormungandr_bin},
        FragmentSenderSetup,
    },
    Version,
};

use super::{LEADER_1, LEADER_2, LEADER_3, LEADER_4};
use std::borrow::Cow;
use std::path::PathBuf;

fn ordinal_suffix(n: u32) -> &'static str {
    match n {
        1 => "st",
        2 => "nd",
        3 => "rd",
        _ => "th",
    }
}

pub fn last_nth_release_title(n: u32) -> Cow<'static, str> {
    match n {
        1 => "legacy_last_release".into(),
        _ => format!("legacy_last_{}{}_release", n, ordinal_suffix(n)).into(),
    }
}

pub fn last_nth_release(context: Context, n: u32) -> Result<ScenarioResult> {
    let title = last_nth_release_title(n);
    let releases = download_last_n_releases(n);
    let last_release = releases.last().unwrap();
    let legacy_app = get_jormungandr_bin(last_release, &context.child_directory(&*title));
    test_legacy_release(context, legacy_app, last_release.version(), title)
}

fn test_legacy_release(
    context: Context,
    legacy_app: PathBuf,
    version: Version,
    name: impl AsRef<str>,
) -> Result<ScenarioResult> {
    let name = name.as_ref();
    let scenario_settings = prepare_scenario! {
        name,
        &mut context,
        topology [
            LEADER_3,
            LEADER_1 -> LEADER_3,LEADER_4,
            LEADER_2 -> LEADER_1,
            LEADER_4 -> LEADER_2,LEADER_3,
        ]
        blockchain {
            consensus = GenesisPraos,
            number_of_slots_per_epoch = 60,
            slot_duration = 1,
            leaders = [ LEADER_1 ],
            initials = [
                "account" "unassigned1" with   500_000_000,
                "account" "delegated1" with  2_000_000_000 delegates to LEADER_1,
                "account" "delegated2" with  2_000_000_000 delegates to LEADER_2,
                "account" "delegated3" with  2_000_000_000 delegates to LEADER_3,
                "account" "delegated4" with  2_000_000_000 delegates to LEADER_4
            ],
        }
    };

    let mut controller = scenario_settings.build(context)?;

    controller.monitor_nodes();
    let mut leader3 =
        controller.spawn_node(LEADER_3, LeadershipMode::Leader, PersistenceMode::InMemory)?;
    leader3.wait_for_bootstrap()?;
    let mut leader1 = controller.spawn_legacy_node(
        controller
            .new_spawn_params(LEADER_1)
            .persistence_mode(PersistenceMode::Persistent)
            .jormungandr(legacy_app),
        &version,
    )?;
    leader1.wait_for_bootstrap()?;
    let mut leader2 =
        controller.spawn_node(LEADER_2, LeadershipMode::Leader, PersistenceMode::InMemory)?;

    leader2.wait_for_bootstrap()?;
    let mut leader4 =
        controller.spawn_node(LEADER_4, LeadershipMode::Leader, PersistenceMode::InMemory)?;
    leader4.wait_for_bootstrap()?;

    let mut wallet1 = controller.wallet("unassigned1")?;
    let mut wallet2 = controller.wallet("delegated1")?;

    FragmentSender::from(controller.settings()).send_transactions_round_trip(
        10,
        &mut wallet1,
        &mut wallet2,
        &leader2,
        1_000.into(),
    )?;

    utils::measure_and_log_sync_time(
        &[
            &leader1 as &dyn SyncNode,
            &leader2 as &dyn SyncNode,
            &leader3 as &dyn SyncNode,
            &leader4 as &dyn SyncNode,
        ],
        SyncWaitParams::network_size(4, 2).into(),
        name,
        MeasurementReportInterval::Standard,
    )?;

    leader4.shutdown()?;
    leader3.shutdown()?;
    leader2.shutdown()?;
    leader1.shutdown()?;

    controller.finalize();
    Ok(ScenarioResult::passed(name))
}

pub fn disruption_last_nth_release_title(n: u32) -> Cow<'static, str> {
    match n {
        1 => "legacy_disruption_last_release".into(),
        _ => format!("legacy_disruption_last_{}{}_release", n, ordinal_suffix(n)).into(),
    }
}

pub fn disruption_last_nth_release(context: Context, n: u32) -> Result<ScenarioResult> {
    let title = disruption_last_nth_release_title(n);
    let releases = download_last_n_releases(n);
    let last_release = releases.last().unwrap();
    let legacy_app = get_jormungandr_bin(last_release, &context.child_directory(&*title));
    test_legacy_disruption_release(context, legacy_app, last_release.version(), title)
}

fn test_legacy_disruption_release(
    context: Context,
    legacy_app: PathBuf,
    version: Version,
    name: impl AsRef<str>,
) -> Result<ScenarioResult> {
    let name = name.as_ref();
    let scenario_settings = prepare_scenario! {
        name,
        &mut context,
        topology [
            LEADER_1,
            LEADER_2 -> LEADER_1,
            LEADER_3 -> LEADER_1,
            LEADER_4 -> LEADER_1,
        ]
        blockchain {
            consensus = GenesisPraos,
            number_of_slots_per_epoch = 60,
            slot_duration = 1,
            leaders = [ LEADER_1 ],
            initials = [
                "account" "unassigned1" with   500_000_000,
                "account" "delegated1" with  2_000_000_000 delegates to LEADER_1,
                "account" "delegated2" with  2_000_000_000 delegates to LEADER_2,
                "account" "delegated3" with  2_000_000_000 delegates to LEADER_3,
                "account" "delegated4" with  2_000_000_000 delegates to LEADER_4
            ],
        }
    };

    let mut controller = scenario_settings.build(context)?;

    controller.monitor_nodes();

    let mut leader1 = controller.spawn_legacy_node(
        controller
            .new_spawn_params(LEADER_1)
            .persistence_mode(PersistenceMode::Persistent)
            .jormungandr(legacy_app.clone()),
        &version,
    )?;
    leader1.wait_for_bootstrap()?;

    let mut leader2 = controller.spawn_node(
        LEADER_2,
        LeadershipMode::Leader,
        PersistenceMode::Persistent,
    )?;
    leader2.wait_for_bootstrap()?;

    let mut leader3 = controller.spawn_node(
        LEADER_3,
        LeadershipMode::Leader,
        PersistenceMode::Persistent,
    )?;
    leader3.wait_for_bootstrap()?;

    let mut leader4 = controller.spawn_node(
        LEADER_4,
        LeadershipMode::Leader,
        PersistenceMode::Persistent,
    )?;
    leader4.wait_for_bootstrap()?;

    let mut wallet1 = controller.wallet("unassigned1")?;
    let mut wallet2 = controller.wallet("delegated1")?;

    let sender = FragmentSender::from(controller.settings());
    sender.send_transactions_round_trip(10, &mut wallet1, &mut wallet2, &leader2, 1_000.into())?;

    leader4 =
        controller.restart_node(leader4, LeadershipMode::Leader, PersistenceMode::Persistent)?;

    sender
        .clone_with_setup(FragmentSenderSetup::resend_3_times_and_sync_with(vec![
            &leader2,
        ]))
        .send_transactions_round_trip(10, &mut wallet1, &mut wallet2, &leader3, 1_000.into())?;

    leader1.shutdown()?;
    leader1 = controller.spawn_legacy_node(
        controller
            .new_spawn_params(LEADER_1)
            .persistence_mode(PersistenceMode::Persistent)
            .jormungandr(legacy_app),
        &version,
    )?;
    leader1.wait_for_bootstrap()?;

    sender.send_transactions_round_trip(10, &mut wallet1, &mut wallet2, &leader2, 1_000.into())?;

    utils::measure_and_log_sync_time(
        &[
            &leader1 as &dyn SyncNode,
            &leader2 as &dyn SyncNode,
            &leader3 as &dyn SyncNode,
            &leader4 as &dyn SyncNode,
        ],
        SyncWaitParams::network_size(4, 2).into(),
        name,
        MeasurementReportInterval::Standard,
    )?;

    leader4.shutdown()?;
    leader3.shutdown()?;
    leader2.shutdown()?;
    leader1.shutdown()?;

    controller.finalize();
    Ok(ScenarioResult::passed(name))
}

pub fn newest_node_enters_legacy_network(context: Context) -> Result<ScenarioResult> {
    let title = last_nth_release_title(1);
    let releases = download_last_n_releases(1);
    let last_release = releases.last().unwrap();
    let legacy_app = get_jormungandr_bin(last_release, &context.child_directory(&*title));

    let scenario_settings = prepare_scenario! {
        &title,
        &mut context,
        topology [
            LEADER_1,
            LEADER_2 -> LEADER_1,
            LEADER_3 -> LEADER_1,
            LEADER_4 -> LEADER_1,
        ]
        blockchain {
            consensus = GenesisPraos,
            number_of_slots_per_epoch = 60,
            slot_duration = 1,
            leaders = [ LEADER_1 ],
            initials = [
                "account" "unassigned1" with   500_000_000,
                "account" "delegated1" with  2_000_000_000 delegates to LEADER_1,
                "account" "delegated2" with  2_000_000_000 delegates to LEADER_2,
                "account" "delegated3" with  2_000_000_000 delegates to LEADER_3,
                "account" "delegated4" with  2_000_000_000 delegates to LEADER_4
            ],
        }
    };

    let mut controller = scenario_settings.build(context)?;

    controller.monitor_nodes();

    let mut leader1 = controller.spawn_legacy_node(
        controller
            .new_spawn_params(LEADER_1)
            .persistence_mode(PersistenceMode::Persistent)
            .jormungandr(legacy_app.clone()),
        &last_release.version(),
    )?;
    leader1.wait_for_bootstrap()?;

    let mut leader2 = controller.spawn_legacy_node(
        controller
            .new_spawn_params(LEADER_2)
            .persistence_mode(PersistenceMode::Persistent)
            .jormungandr(legacy_app.clone()),
        &last_release.version(),
    )?;
    leader2.wait_for_bootstrap()?;

    let mut leader3 = controller.spawn_legacy_node(
        controller
            .new_spawn_params(LEADER_3)
            .persistence_mode(PersistenceMode::Persistent)
            .jormungandr(legacy_app.clone()),
        &last_release.version(),
    )?;
    leader3.wait_for_bootstrap()?;

    let mut wallet1 = controller.wallet("unassigned1")?;
    let mut wallet2 = controller.wallet("delegated1")?;

    // do some transaction and allow network to spin off a bit
    let sender = FragmentSender::from(controller.settings())
        .clone_with_setup(FragmentSenderSetup::resend_3_times());
    sender.send_transactions_round_trip(10, &mut wallet1, &mut wallet2, &leader2, 1_000.into())?;

    // new node enters the network
    let mut leader4 = controller.spawn_node(
        LEADER_4,
        LeadershipMode::Leader,
        PersistenceMode::Persistent,
    )?;
    leader4.wait_for_bootstrap()?;

    // force newest node to keep up and talk to legacy nodes
    let sender = FragmentSender::from(controller.settings()).clone_with_setup(
        FragmentSenderSetup::resend_3_times_and_sync_with(vec![&leader2]),
    );

    sender.send_transactions_round_trip(10, &mut wallet1, &mut wallet2, &leader3, 1_000.into())?;

    utils::measure_and_log_sync_time(
        &[
            &leader1 as &dyn SyncNode,
            &leader2 as &dyn SyncNode,
            &leader3 as &dyn SyncNode,
            &leader4 as &dyn SyncNode,
        ],
        SyncWaitParams::network_size(4, 2).into(),
        &title,
        MeasurementReportInterval::Standard,
    )?;

    leader4.shutdown()?;

    //let assume that we are not satisfied how newest node behaves and we want to rollback
    let mut old_leader4 = controller.spawn_legacy_node(
        controller
            .new_spawn_params(LEADER_4)
            .persistence_mode(PersistenceMode::Persistent)
            .jormungandr(legacy_app),
        &last_release.version(),
    )?;
    old_leader4.wait_for_bootstrap()?;

    // repeat sync
    sender.send_transactions_round_trip(10, &mut wallet1, &mut wallet2, &leader3, 1_000.into())?;

    utils::measure_and_log_sync_time(
        &[&leader1, &leader2, &leader3, &old_leader4],
        SyncWaitParams::network_size(4, 2).into(),
        &title,
        MeasurementReportInterval::Standard,
    )?;

    old_leader4.shutdown()?;
    leader3.shutdown()?;
    leader2.shutdown()?;
    leader1.shutdown()?;

    controller.finalize();
    Ok(ScenarioResult::passed(title))
}
