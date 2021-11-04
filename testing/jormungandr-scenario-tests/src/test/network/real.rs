use crate::{
    node::{LeadershipMode, PersistenceMode},
    scenario::{
        repository::ScenarioResult, ActiveSlotCoefficient, ConsensusVersion, ControllerBuilder,
        Milli, Node, SlotDuration, Value,
    },
    test::{
        utils::{self, MeasurementReportInterval, SyncNode, SyncWaitParams},
        Result,
    },
    Context, Node as NodeController,
};
use jormungandr_testing_utils::testing::{
    network::{Blockchain, Topology, WalletTemplate},
    node::{download_last_n_releases, get_jormungandr_bin},
};

use rand_chacha::ChaChaRng;

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
    title: &str,
    relay_nodes_count: u32,
    nodes_count_per_relay: u32,
    legacy_nodes_count_per_relay: u32,
    consensus: ConsensusVersion,
) -> ControllerBuilder {
    let mut builder = ControllerBuilder::new(title);
    let mut topology = Topology::default().with_node(Node::new(CORE_NODE));

    let mut blockchain = Blockchain::default();
    blockchain.set_consensus(consensus);
    blockchain.set_slot_duration(SlotDuration::new(1).unwrap());
    blockchain.set_consensus_genesis_praos_active_slot_coeff(
        ActiveSlotCoefficient::new(Milli::from_millis(700)).unwrap(),
    );

    for i in 0..relay_nodes_count {
        let relay_name = relay_name(i + 1);
        topology = topology.with_node(Node::new(&relay_name).with_trusted_peer(CORE_NODE));
        blockchain.add_leader(relay_name);
    }

    let mut leader_counter = 1;
    let mut legacy_nodes_counter = 1;

    for i in 0..relay_nodes_count {
        let relay_name = relay_name(i + 1);

        for _ in 0..nodes_count_per_relay {
            let leader_name = leader_name(leader_counter);
            topology = topology.with_node(Node::new(&leader_name).with_trusted_peer(&relay_name));

            blockchain.add_leader(leader_name);

            leader_counter += 1;
        }

        for _ in 0..legacy_nodes_count_per_relay {
            let legacy_name = legacy_name(legacy_nodes_counter);
            topology = topology.with_node(Node::new(&legacy_name).with_trusted_peer(&relay_name));

            blockchain.add_leader(legacy_name);

            legacy_nodes_counter += 1;
        }
    }

    builder = builder.topology(topology);

    // adds all nodes as leaders
    blockchain.add_leader(CORE_NODE);

    for i in 1..leader_counter {
        let initial_wallet_name = wallet_name(i);
        let mut wallet = WalletTemplate::new_account(
            initial_wallet_name.to_owned(),
            Value(100_000),
            blockchain.discrimination(),
        );
        *wallet.delegate_mut() = Some(leader_name(i).to_owned());
        blockchain.add_wallet(wallet);
    }

    for i in 1..legacy_nodes_counter {
        let initial_wallet_name = wallet_name(i);
        let mut wallet = WalletTemplate::new_account(
            initial_wallet_name.to_owned(),
            Value(100_000),
            blockchain.discrimination(),
        );
        *wallet.delegate_mut() = Some(legacy_name(i).to_owned());
        blockchain.add_wallet(wallet);
    }

    builder.blockchain(blockchain)
}

pub fn real_praos_network(context: Context<ChaChaRng>) -> Result<ScenarioResult> {
    let relay_nodes_count = 3;
    let leaders_per_relay = 11;
    let legacies_per_relay = 0;
    let name = "real_praos_Network".to_owned();

    real_network(
        relay_nodes_count,
        leaders_per_relay,
        legacies_per_relay,
        context,
        ConsensusVersion::GenesisPraos,
        PersistenceMode::Persistent,
        name,
    )
}

pub fn real_bft_network(context: Context<ChaChaRng>) -> Result<ScenarioResult> {
    let relay_nodes_count = 3;
    let leaders_per_relay = 11;
    let legacies_per_relay = 0;
    let name = "real_bft_Network".to_owned();

    real_network(
        relay_nodes_count,
        leaders_per_relay,
        legacies_per_relay,
        context,
        ConsensusVersion::Bft,
        PersistenceMode::Persistent,
        name,
    )
}

pub fn real_network(
    relay_nodes_count: u32,
    leaders_per_relay: u32,
    legacies_per_relay: u32,
    context: Context<ChaChaRng>,
    consensus: ConsensusVersion,
    persistence_mode: PersistenceMode,
    name: String,
) -> Result<ScenarioResult> {
    let scenario_settings = prepare_real_scenario(
        &name,
        relay_nodes_count,
        leaders_per_relay,
        legacies_per_relay,
        consensus,
    );
    let mut controller = scenario_settings.build(context)?;

    let mut core = controller.spawn_node(CORE_NODE, LeadershipMode::Leader, persistence_mode)?;

    let mut relays = vec![];
    for i in 0..relay_nodes_count {
        relays.push(controller.spawn_node(
            &relay_name(i + 1),
            LeadershipMode::Leader,
            persistence_mode,
        )?);
    }

    let mut leaders = vec![];
    for i in 0..(relay_nodes_count * leaders_per_relay) {
        leaders.push(controller.spawn_node(
            &leader_name(i + 1),
            LeadershipMode::Leader,
            persistence_mode,
        )?);
    }

    let releases = download_last_n_releases(1);
    let last_release = releases.last().unwrap();
    let legacy_app = get_jormungandr_bin(last_release, controller.working_directory());
    let version = last_release.version();

    let mut legacy_leaders = vec![];

    for i in 0..(relay_nodes_count * legacies_per_relay) {
        legacy_leaders.push(
            controller.spawn_legacy_node(
                controller
                    .new_spawn_params(&legacy_name(i + 1))
                    .leadership_mode(LeadershipMode::Leader)
                    .persistence_mode(persistence_mode)
                    .jormungandr(legacy_app.clone()),
                &version,
            )?,
        );
    }

    controller.monitor_nodes();
    core.wait_for_bootstrap()?;
    leaders.last().unwrap().wait_for_bootstrap()?;

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
    )?;

    let mut wallet = controller.wallet(&wallet_name(1)).unwrap();
    let wallet2 = controller.wallet(&wallet_name(2)).unwrap();

    let fragment_nodes: Vec<&NodeController> = leaders.iter().collect();

    utils::measure_single_transaction_propagation_speed(
        &mut controller,
        &mut wallet,
        &wallet2,
        &fragment_nodes,
        SyncWaitParams::large_network(leaders_count).into(),
        &format!("{}_single_transaction_propagation", name),
        MeasurementReportInterval::Standard,
    )?;

    core.shutdown()?;
    controller.finalize();
    Ok(ScenarioResult::passed(name))
}
