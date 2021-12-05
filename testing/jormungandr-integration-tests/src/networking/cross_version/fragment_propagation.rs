use super::{ALICE, BOB, CLARICE, DAVID};
use super::{LEADER, PASSIVE};
use function_name::named;
use hersir::controller::Context;
use jormungandr_testing_utils::testing::network::builder::NetworkBuilder;
use jormungandr_testing_utils::testing::network::controller::Controller;
use jormungandr_testing_utils::testing::network::wallet::template::builder::WalletTemplateBuilder;
use jormungandr_testing_utils::testing::network::Node;
use jormungandr_testing_utils::testing::network::SpawnParams;
use jormungandr_testing_utils::testing::network::Topology;
use jormungandr_testing_utils::testing::FragmentSender;
use jormungandr_testing_utils::{
    stake_pool::StakePool,
    testing::{
        node::{download_last_n_releases, get_jormungandr_bin},
        FragmentNode, SyncNode,
    },
    version_0_8_19, Version,
};
use std::path::PathBuf;

#[test]
#[named]
pub fn legacy_current_node_fragment_propagation() {
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
    let mut context = Context::default();
    let (legacy_app, version) = get_legacy_data(function_name!(), &mut context);

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
#[named]
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
    let mut context = Context::default();
    let (legacy_app, version) = get_legacy_data(function_name!(), &mut context);

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

fn get_legacy_data(title: &str, context: &mut Context) -> (PathBuf, Version) {
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
    let mut alice = controller.wallet(ALICE).unwrap();
    let mut bob = controller.wallet(BOB).unwrap();
    let clarice = controller.wallet(CLARICE).unwrap();
    let mut david = controller.wallet(DAVID).unwrap();

    let leader_stake_pool = controller.stake_pool(LEADER).unwrap();
    let david_stake_pool = StakePool::new(&david);

    let sender = FragmentSender::from(controller.settings());

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
