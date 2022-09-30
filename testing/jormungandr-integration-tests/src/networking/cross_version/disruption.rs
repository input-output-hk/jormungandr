use super::{ALICE, BOB, LEADER_1, LEADER_2, LEADER_3, LEADER_4};
use crate::networking::utils;
use assert_fs::fixture::PathChild;
use function_name::named;
use hersir::{
    builder::{NetworkBuilder, Node, Topology},
    config::{Blockchain, SessionSettings, SpawnParams, WalletTemplateBuilder},
};
use jormungandr_automation::{
    jormungandr::{download_last_n_releases, get_jormungandr_bin, Version},
    testing::{benchmark::MeasurementReportInterval, SyncNode, SyncWaitParams},
};
use rstest::rstest;
use std::path::PathBuf;
use thor::{FragmentSender, FragmentSenderSetup};

#[rstest]
#[case(0)]
#[case(1)]
#[case(2)]
#[case(3)]
#[case(4)]
#[case(5)]
pub fn last_nth_release(#[case] n: u32) {
    let session_settings = SessionSettings::default();
    let releases = download_last_n_releases(n);
    let last_release = releases.last().unwrap();
    let legacy_app = get_jormungandr_bin(last_release, &session_settings.root.child("jormungandr"));
    test_legacy_release(legacy_app, last_release.version())
}

#[named]
fn test_legacy_release(legacy_app: PathBuf, version: Version) {
    let mut controller = NetworkBuilder::default()
        .topology(
            Topology::default()
                .with_node(Node::new(LEADER_3))
                .with_node(
                    Node::new(LEADER_1)
                        .with_trusted_peer(LEADER_3)
                        .with_trusted_peer(LEADER_4),
                )
                .with_node(Node::new(LEADER_2).with_trusted_peer(LEADER_1))
                .with_node(
                    Node::new(LEADER_4)
                        .with_trusted_peer(LEADER_2)
                        .with_trusted_peer(LEADER_3),
                ),
        )
        .wallet_template(
            WalletTemplateBuilder::new(ALICE)
                .with(2_500_000_000)
                .delegated_to(LEADER_2)
                .build(),
        )
        .wallet_template(
            WalletTemplateBuilder::new(BOB)
                .with(2_000_000_000)
                .delegated_to(LEADER_1)
                .build(),
        )
        .build()
        .unwrap();

    let leader3 = controller
        .spawn(SpawnParams::new(LEADER_2).in_memory())
        .unwrap();

    let (leader1, _) = controller
        .spawn_legacy(SpawnParams::new(LEADER_1).jormungandr(legacy_app), &version)
        .unwrap();
    let leader2 = controller
        .spawn(SpawnParams::new(LEADER_2).in_memory())
        .unwrap();
    let leader4 = controller
        .spawn(SpawnParams::new(LEADER_4).in_memory())
        .unwrap();

    let mut wallet1 = controller.controlled_wallet(ALICE).unwrap();
    let mut wallet2 = controller.controlled_wallet(BOB).unwrap();

    FragmentSender::from(&controller.settings().block0)
        .send_transactions_round_trip(10, &mut wallet1, &mut wallet2, &leader2, 1_000.into())
        .unwrap();

    utils::measure_and_log_sync_time(
        &[
            &leader1 as &dyn SyncNode,
            &leader2 as &dyn SyncNode,
            &leader3 as &dyn SyncNode,
            &leader4 as &dyn SyncNode,
        ],
        SyncWaitParams::network_size(4, 2).into(),
        &format!("{}_{}", function_name!(), version),
        MeasurementReportInterval::Standard,
    )
    .unwrap();
}

#[rstest]
#[case(0)]
#[case(1)]
#[case(2)]
#[case(3)]
#[case(4)]
#[case(5)]
pub fn disruption_last_nth_release(#[case] n: u32) {
    let session_settings = SessionSettings::default();
    let releases = download_last_n_releases(n);
    let last_release = releases.last().unwrap();
    let legacy_app = get_jormungandr_bin(last_release, &session_settings.root.child("jormungandr"));
    test_legacy_disruption_release(legacy_app, last_release.version())
}

#[named]
fn test_legacy_disruption_release(legacy_app: PathBuf, version: Version) {
    let mut controller = NetworkBuilder::default()
        .topology(
            Topology::default()
                .with_node(Node::new(LEADER_1))
                .with_node(Node::new(LEADER_2).with_trusted_peer(LEADER_1))
                .with_node(Node::new(LEADER_3).with_trusted_peer(LEADER_1))
                .with_node(Node::new(LEADER_4).with_trusted_peer(LEADER_1)),
        )
        .wallet_template(
            WalletTemplateBuilder::new(ALICE)
                .with(2_500_000_000)
                .delegated_to(LEADER_2)
                .build(),
        )
        .wallet_template(
            WalletTemplateBuilder::new(BOB)
                .with(2_000_000_000)
                .delegated_to(LEADER_1)
                .build(),
        )
        .build()
        .unwrap();

    let (leader1, _) = controller
        .spawn_legacy(
            SpawnParams::new(LEADER_1).jormungandr(legacy_app.clone()),
            &version,
        )
        .unwrap();

    let leader2 = controller.spawn(SpawnParams::new(LEADER_2)).unwrap();
    let leader3 = controller.spawn(SpawnParams::new(LEADER_3)).unwrap();
    let mut leader4 = controller.spawn(SpawnParams::new(LEADER_4)).unwrap();

    let mut wallet1 = controller.controlled_wallet(ALICE).unwrap();
    let mut wallet2 = controller.controlled_wallet(BOB).unwrap();

    let sender = FragmentSender::from(&controller.settings().block0);
    sender
        .send_transactions_round_trip(10, &mut wallet1, &mut wallet2, &leader2, 1_000.into())
        .unwrap();

    leader4.shutdown();
    leader4 = controller.spawn(SpawnParams::new(LEADER_4)).unwrap();

    sender
        .clone_with_setup(FragmentSenderSetup::resend_3_times_and_sync_with(vec![
            &leader2,
        ]))
        .send_transactions_round_trip(10, &mut wallet1, &mut wallet2, &leader3, 1_000.into())
        .unwrap();

    leader1.shutdown();
    let (leader1, _) = controller
        .spawn_legacy(SpawnParams::new(LEADER_1).jormungandr(legacy_app), &version)
        .unwrap();

    sender
        .send_transactions_round_trip(10, &mut wallet1, &mut wallet2, &leader2, 1_000.into())
        .unwrap();

    utils::measure_and_log_sync_time(
        &[
            &leader1 as &dyn SyncNode,
            &leader2 as &dyn SyncNode,
            &leader3 as &dyn SyncNode,
            &leader4 as &dyn SyncNode,
        ],
        SyncWaitParams::network_size(4, 2).into(),
        &format!("{}_{}", function_name!(), version),
        MeasurementReportInterval::Standard,
    )
    .unwrap();
}

#[test]
#[named]
pub fn newest_node_enters_legacy_network() {
    let title = function_name!();
    let releases = download_last_n_releases(1);
    let last_release = releases.last().unwrap();
    let session_settings = SessionSettings::default();
    let legacy_app = get_jormungandr_bin(last_release, &session_settings.root.child("jormungandr"));

    let mut controller = NetworkBuilder::default()
        .topology(
            Topology::default()
                .with_node(Node::new(LEADER_1))
                .with_node(Node::new(LEADER_2).with_trusted_peer(LEADER_1))
                .with_node(Node::new(LEADER_3).with_trusted_peer(LEADER_1))
                .with_node(Node::new(LEADER_4).with_trusted_peer(LEADER_1)),
        )
        .blockchain_config(
            Blockchain::default().with_leaders(vec![LEADER_1, LEADER_2, LEADER_3, LEADER_4]),
        )
        .wallet_template(
            WalletTemplateBuilder::new(ALICE)
                .with(2_500_000_000)
                .delegated_to(LEADER_2)
                .build(),
        )
        .wallet_template(
            WalletTemplateBuilder::new(BOB)
                .with(2_000_000_000)
                .delegated_to(LEADER_1)
                .build(),
        )
        .build()
        .unwrap();

    let (leader1, _) = controller
        .spawn_legacy(
            SpawnParams::new(LEADER_1).jormungandr(legacy_app.clone()),
            &last_release.version(),
        )
        .unwrap();

    let (leader2, _) = controller
        .spawn_legacy(
            SpawnParams::new(LEADER_2).jormungandr(legacy_app.clone()),
            &last_release.version(),
        )
        .unwrap();

    let (leader3, _) = controller
        .spawn_legacy(
            SpawnParams::new(LEADER_3).jormungandr(legacy_app.clone()),
            &last_release.version(),
        )
        .unwrap();

    let mut wallet1 = controller.controlled_wallet(ALICE).unwrap();
    let mut wallet2 = controller.controlled_wallet(BOB).unwrap();

    // do some transaction and allow network to spin off a bit
    let sender = FragmentSender::from(&controller.settings().block0)
        .clone_with_setup(FragmentSenderSetup::resend_3_times());
    sender
        .send_transactions_round_trip(10, &mut wallet1, &mut wallet2, &leader2, 1_000.into())
        .unwrap();

    // new node enters the network
    let leader4 = controller
        .spawn(SpawnParams::new(LEADER_4).in_memory())
        .unwrap();

    // force newest node to keep up and talk to legacy nodes
    let sender = FragmentSender::from(&controller.settings().block0).clone_with_setup(
        FragmentSenderSetup::resend_3_times_and_sync_with(vec![&leader2]),
    );

    sender
        .send_transactions_round_trip(10, &mut wallet1, &mut wallet2, &leader3, 1_000.into())
        .unwrap();

    utils::measure_and_log_sync_time(
        &[
            &leader1 as &dyn SyncNode,
            &leader2 as &dyn SyncNode,
            &leader3 as &dyn SyncNode,
            &leader4 as &dyn SyncNode,
        ],
        SyncWaitParams::network_size(4, 2).into(),
        title,
        MeasurementReportInterval::Standard,
    )
    .unwrap();

    leader4.shutdown();

    //let assume that we are not satisfied how newest node behaves and we want to rollback
    let (old_leader4, _) = controller
        .spawn_legacy(
            SpawnParams::new(LEADER_4).jormungandr(legacy_app),
            &last_release.version(),
        )
        .unwrap();

    // repeat sync
    sender
        .send_transactions_round_trip(10, &mut wallet1, &mut wallet2, &leader3, 1_000.into())
        .unwrap();

    utils::measure_and_log_sync_time(
        &[&leader1, &leader2, &leader3, &old_leader4],
        SyncWaitParams::network_size(4, 2).into(),
        title,
        MeasurementReportInterval::Standard,
    )
    .unwrap();
}
