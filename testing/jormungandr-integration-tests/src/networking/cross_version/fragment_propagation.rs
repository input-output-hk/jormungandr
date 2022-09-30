use super::{ALICE, BOB, CLARICE, DAVID, LEADER, PASSIVE};
use hersir::{
    builder::{NetworkBuilder, Node, Topology},
    config::{Blockchain, SessionSettings, SpawnParams, WalletTemplateBuilder},
    controller::Controller,
};
use jormungandr_automation::{
    jormungandr::{
        download_last_n_releases, get_jormungandr_bin, version_0_8_19, FragmentNode, Version,
    },
    testing::SyncNode,
};
use std::path::PathBuf;
use thor::{FragmentSender, StakePool};

#[test]
pub fn legacy_current_node_fragment_propagation() {
    let mut controller = NetworkBuilder::default()
        .topology(
            Topology::default()
                .with_node(Node::new(LEADER))
                .with_node(Node::new(PASSIVE).with_trusted_peer(LEADER)),
        )
        .blockchain_config(Blockchain::default().with_leader(LEADER))
        .wallet_template(
            WalletTemplateBuilder::new(ALICE)
                .with(2_500_000_000)
                .delegated_to(LEADER)
                .build(),
        )
        .wallet_template(WalletTemplateBuilder::new(BOB).with(2_000_000_000).build())
        .wallet_template(
            WalletTemplateBuilder::new(CLARICE)
                .with(2_000_000_000)
                .build(),
        )
        .wallet_template(
            WalletTemplateBuilder::new(DAVID)
                .with(2_000_000_000)
                .build(),
        )
        .build()
        .unwrap();
    let session_settings = SessionSettings::default();
    let (legacy_app, version) = get_legacy_data(&session_settings);

    let _leader = controller
        .spawn(SpawnParams::new(LEADER).in_memory())
        .unwrap();

    let (passive, _) = controller
        .spawn_legacy(
            SpawnParams::new(PASSIVE)
                .in_memory()
                .jormungandr(legacy_app),
            &version,
        )
        .unwrap();

    send_all_fragment_types(&mut controller, &passive, Some(version));
}

#[test]
pub fn current_node_legacy_fragment_propagation() {
    let mut controller = NetworkBuilder::default()
        .topology(
            Topology::default()
                .with_node(Node::new(LEADER))
                .with_node(Node::new(PASSIVE).with_trusted_peer(LEADER)),
        )
        .wallet_template(
            WalletTemplateBuilder::new(ALICE)
                .with(2_500_000_000)
                .delegated_to(LEADER)
                .build(),
        )
        .wallet_template(WalletTemplateBuilder::new(BOB).with(2_000_000_000).build())
        .wallet_template(
            WalletTemplateBuilder::new(CLARICE)
                .with(2_000_000_000)
                .build(),
        )
        .wallet_template(
            WalletTemplateBuilder::new(DAVID)
                .with(2_000_000_000)
                .build(),
        )
        .build()
        .unwrap();
    let session_settings = SessionSettings::default();
    let (legacy_app, version) = get_legacy_data(&session_settings);

    let _leader = controller
        .spawn_legacy(
            SpawnParams::new(LEADER).in_memory().jormungandr(legacy_app),
            &version,
        )
        .unwrap();

    let passive = controller
        .spawn(SpawnParams::new(PASSIVE).in_memory())
        .unwrap();

    send_all_fragment_types(&mut controller, &passive, Some(version));
}

#[test]
pub fn current_node_fragment_propagation() {
    let mut controller = NetworkBuilder::default()
        .topology(
            Topology::default()
                .with_node(Node::new(LEADER))
                .with_node(Node::new(PASSIVE).with_trusted_peer(LEADER)),
        )
        .wallet_template(
            WalletTemplateBuilder::new(ALICE)
                .with(2_500_000_000)
                .delegated_to(LEADER)
                .build(),
        )
        .wallet_template(WalletTemplateBuilder::new(BOB).with(2_000_000_000).build())
        .wallet_template(
            WalletTemplateBuilder::new(CLARICE)
                .with(2_000_000_000)
                .build(),
        )
        .wallet_template(
            WalletTemplateBuilder::new(DAVID)
                .with(2_000_000_000)
                .build(),
        )
        .build()
        .unwrap();

    let _leader = controller
        .spawn(SpawnParams::new(LEADER).in_memory())
        .unwrap();

    let passive = controller
        .spawn(SpawnParams::new(PASSIVE).in_memory())
        .unwrap();

    send_all_fragment_types(&mut controller, &passive, None);
}

fn get_legacy_data(session_settings: &SessionSettings) -> (PathBuf, Version) {
    let releases = download_last_n_releases(1);
    let last_release = releases.last().unwrap();
    let legacy_app = get_jormungandr_bin(last_release, &session_settings.root);
    (legacy_app, last_release.version())
}

fn send_all_fragment_types<A: FragmentNode + SyncNode + Sized + Send>(
    controller: &mut Controller,
    passive: &A,
    version: Option<Version>,
) {
    let mut alice = controller.controlled_wallet(ALICE).unwrap();
    let mut bob = controller.controlled_wallet(BOB).unwrap();
    let clarice = controller.controlled_wallet(CLARICE).unwrap();
    let mut david = controller.controlled_wallet(DAVID).unwrap();

    let leader_stake_pool = controller.stake_pool(LEADER).unwrap();
    let david_stake_pool = StakePool::new(&david);

    let sender = FragmentSender::from(&controller.settings().block0);

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
        .send_full_delegation(&mut bob, leader_stake_pool, passive)
        .expect("send full delegation failed");

    let distribution: Vec<(&StakePool, u8)> = vec![(leader_stake_pool, 1), (&david_stake_pool, 1)];
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
