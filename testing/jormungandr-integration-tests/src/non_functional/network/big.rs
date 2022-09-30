use crate::networking::utils;
use chain_impl_mockchain::{chaintypes::ConsensusVersion, milli::Milli, value::Value};
use function_name::named;
use hersir::{
    builder::{NetworkBuilder, Node, Topology},
    config::{Blockchain, SpawnParams, WalletTemplate},
    controller::Controller,
};
use jormungandr_automation::{
    jormungandr::{
        download_last_n_releases, get_jormungandr_bin, JormungandrProcess, PersistenceMode,
    },
    testing::{benchmark::MeasurementReportInterval, SyncNode, SyncWaitParams},
};
use jormungandr_lib::interfaces::{ActiveSlotCoefficient, SlotDuration};
use std::collections::HashMap;

const CORE_NODE: &str = "Core";
const RELAY_NODE: &str = "Relay";
const LEADER_NODE: &str = "Leader";
const LEGACY_NODE: &str = "Legacy";

fn relay_name(i: u32) -> String {
    format!("{}_{}", RELAY_NODE, i)
}

fn leader_name(i: u32) -> String {
    format!("{}_{}", LEADER_NODE, i)
}

fn legacy_name(i: u32) -> String {
    format!("{}_{}", LEGACY_NODE, i)
}

fn wallet_name(i: u32) -> String {
    format!("leader_wallet_{}", i)
}

fn prepare_real_scenario(
    relay_nodes_count: u32,
    nodes_count_per_relay: u32,
    legacy_nodes_count_per_relay: u32,
    consensus: ConsensusVersion,
) -> Controller {
    let mut builder = NetworkBuilder::default();
    let mut topology = Topology::default().with_node(Node::new(CORE_NODE));

    let mut blockchain = Blockchain::default()
        .with_consensus(consensus)
        .with_slot_duration(SlotDuration::new(1).unwrap())
        .with_consensus_genesis_praos_active_slot_coeff(
            ActiveSlotCoefficient::new(Milli::from_millis(700)).unwrap(),
        );

    for i in 0..relay_nodes_count {
        let relay_name = relay_name(i + 1);
        topology = topology.with_node(Node::new(&relay_name).with_trusted_peer(CORE_NODE));
        blockchain = blockchain.with_leader(relay_name);
    }

    let mut leader_counter = 1;
    let mut legacy_nodes_counter = 1;

    for i in 0..relay_nodes_count {
        let relay_name = relay_name(i + 1);

        for _ in 0..nodes_count_per_relay {
            let leader_name = leader_name(leader_counter);
            topology = topology.with_node(Node::new(&leader_name).with_trusted_peer(&relay_name));

            blockchain = blockchain.with_leader(leader_name);

            leader_counter += 1;
        }

        for _ in 0..legacy_nodes_count_per_relay {
            let legacy_name = legacy_name(legacy_nodes_counter);
            topology = topology.with_node(Node::new(&legacy_name).with_trusted_peer(&relay_name));

            blockchain = blockchain.with_leader(legacy_name);

            legacy_nodes_counter += 1;
        }
    }

    builder = builder.topology(topology);

    // adds all nodes as leaders
    blockchain = blockchain.with_leader(CORE_NODE);

    for i in 1..leader_counter {
        let initial_wallet_name = wallet_name(i);
        let mut wallet = WalletTemplate::new_account(
            initial_wallet_name.to_owned(),
            Value(100_000),
            blockchain.discrimination(),
            HashMap::new(),
        );
        *wallet.delegate_mut() = Some(leader_name(i).to_owned());
        builder = builder.wallet_template(wallet);
    }

    for i in 1..legacy_nodes_counter {
        let initial_wallet_name = wallet_name(i);
        let mut wallet = WalletTemplate::new_account(
            initial_wallet_name.to_owned(),
            Value(100_000),
            blockchain.discrimination(),
            HashMap::new(),
        );
        *wallet.delegate_mut() = Some(legacy_name(i).to_owned());
        builder = builder.wallet_template(wallet);
    }

    builder.blockchain_config(blockchain).build().unwrap()
}

#[test]
#[named]
pub fn real_praos_network() {
    let relay_nodes_count = 3;
    let leaders_per_relay = 11;
    let legacies_per_relay = 0;

    real_network(
        relay_nodes_count,
        leaders_per_relay,
        legacies_per_relay,
        ConsensusVersion::GenesisPraos,
        PersistenceMode::Persistent,
        function_name!(),
    )
}

#[test]
#[named]
pub fn real_bft_network() {
    let relay_nodes_count = 3;
    let leaders_per_relay = 11;
    let legacies_per_relay = 0;

    real_network(
        relay_nodes_count,
        leaders_per_relay,
        legacies_per_relay,
        ConsensusVersion::Bft,
        PersistenceMode::Persistent,
        function_name!(),
    )
}

pub fn real_network(
    relay_nodes_count: u32,
    leaders_per_relay: u32,
    legacies_per_relay: u32,
    consensus: ConsensusVersion,
    persistence_mode: PersistenceMode,
    name: &str,
) {
    let mut controller = prepare_real_scenario(
        relay_nodes_count,
        leaders_per_relay,
        legacies_per_relay,
        consensus,
    );

    let _core = controller.spawn(SpawnParams::new(CORE_NODE)).unwrap();

    let mut relays = vec![];
    for i in 0..relay_nodes_count {
        relays.push(
            controller
                .spawn(SpawnParams::new(&relay_name(i + 1)).persistence_mode(persistence_mode))
                .unwrap(),
        );
    }

    let mut leaders = vec![];
    for i in 0..(relay_nodes_count * leaders_per_relay) {
        leaders.push(
            controller
                .spawn(SpawnParams::new(&leader_name(i + 1)).persistence_mode(persistence_mode))
                .unwrap(),
        );
    }

    let releases = download_last_n_releases(1);
    let last_release = releases.last().unwrap();
    let legacy_app = get_jormungandr_bin(last_release, controller.working_directory());
    let version = last_release.version();

    let mut legacy_leaders = vec![];

    for i in 0..(relay_nodes_count * legacies_per_relay) {
        legacy_leaders.push(
            controller
                .spawn_legacy(
                    SpawnParams::new(&legacy_name(i + 1))
                        .persistence_mode(persistence_mode)
                        .jormungandr(legacy_app.clone()),
                    &version,
                )
                .unwrap()
                .0,
        );
    }

    let mut sync_nodes: Vec<&dyn SyncNode> =
        leaders.iter().map(|node| node as &dyn SyncNode).collect();
    sync_nodes.extend(legacy_leaders.iter().map(|node| node as &dyn SyncNode));

    utils::measure_how_many_nodes_are_running(&sync_nodes, &format!("{}_bootstrap_score", name));

    let leaders_count = leaders.len() as u64;
    utils::measure_and_log_sync_time(
        &sync_nodes,
        SyncWaitParams::large_network(leaders_count).into(),
        &format!("{}_sync", name),
        MeasurementReportInterval::Long,
    )
    .unwrap();

    let mut wallet = controller.controlled_wallet(&wallet_name(1)).unwrap();
    let wallet2 = controller.controlled_wallet(&wallet_name(2)).unwrap();

    let fragment_nodes: Vec<&JormungandrProcess> = leaders.iter().collect();

    utils::measure_single_transaction_propagation_speed(
        &mut controller,
        &mut wallet,
        &wallet2,
        &fragment_nodes,
        SyncWaitParams::large_network(leaders_count).into(),
        &format!("{}_single_transaction_propagation", name),
        MeasurementReportInterval::Standard,
    )
}
