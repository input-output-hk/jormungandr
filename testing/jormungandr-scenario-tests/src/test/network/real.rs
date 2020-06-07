use crate::{
    node::{LeadershipMode, PersistenceMode},
    scenario::{
        repository::ScenarioResult, ActiveSlotCoefficient, ConsensusVersion, ControllerBuilder,
        KESUpdateSpeed, Milli, Node, NumberOfSlotsPerEpoch, SlotDuration, Value,
    },
    test::{
        utils::{self, MeasurementReportInterval, SyncNode, SyncWaitParams},
        Result,
    },
    Context, NodeController,
};
use chain_core::property::FromStr;
use jormungandr_testing_utils::{
    legacy::Version,
    testing::network_builder::{Blockchain, TopologyBuilder, WalletTemplate},
};

use jormungandr_integration_tests::common::legacy::download_last_n_releases;
use jormungandr_integration_tests::common::legacy::get_jormungandr_bin;

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
    context: &Context<ChaChaRng>,
) -> ControllerBuilder {
    let mut builder = ControllerBuilder::new(title);
    let mut topology_builder = TopologyBuilder::new();

    let core_node = Node::new(CORE_NODE);
    topology_builder.register_node(core_node);

    for i in 0..relay_nodes_count {
        let relay_name = relay_name(i + 1);
        let mut relay_node = Node::new(&relay_name);
        relay_node.add_trusted_peer(CORE_NODE);
        topology_builder.register_node(relay_node);
    }

    let mut leader_counter = 1;
    let mut legacy_nodes_counter = 1;

    for i in 0..relay_nodes_count {
        let relay_name = relay_name(i + 1);

        for _ in 0..nodes_count_per_relay {
            let leader_name = leader_name(leader_counter);
            let mut leader_node = Node::new(&leader_name);

            leader_node.add_trusted_peer(relay_name.clone());
            topology_builder.register_node(leader_node);
            leader_counter += 1;
        }

        for _ in 0..legacy_nodes_count_per_relay {
            let mut legacy_node = Node::new(&legacy_name(legacy_nodes_counter));
            legacy_node.add_trusted_peer(relay_name.clone());
            topology_builder.register_node(legacy_node);
            legacy_nodes_counter += 1;
        }
    }

    let topology = topology_builder.build();
    builder.set_topology(topology);

    let mut blockchain = Blockchain::new(
        ConsensusVersion::GenesisPraos,
        NumberOfSlotsPerEpoch::new(60).expect("valid number of slots per epoch"),
        SlotDuration::new(1).expect("valid slot duration in seconds"),
        KESUpdateSpeed::new(46800).expect("valid kes update speed in seconds"),
        ActiveSlotCoefficient::new(Milli::from_millis(700))
            .expect("active slot coefficient in millis"),
    );

    blockchain.add_leader(CORE_NODE);

    for i in 1..leader_counter {
        let initial_wallet_name = wallet_name(i);
        let mut wallet =
            WalletTemplate::new_account(initial_wallet_name.to_owned(), Value(100_000).into());
        *wallet.delegate_mut() = Some(leader_name(i).to_owned());
        blockchain.add_wallet(wallet);
    }

    for i in 1..legacy_nodes_counter {
        let initial_wallet_name = wallet_name(i);
        let mut wallet =
            WalletTemplate::new_account(initial_wallet_name.to_owned(), Value(100_000).into());
        *wallet.delegate_mut() = Some(legacy_name(i).to_owned());
        blockchain.add_wallet(wallet);
    }

    builder.set_blockchain(blockchain);
    builder.build_settings(&mut context.clone());
    builder
}

pub fn real_network(context: Context<ChaChaRng>) -> Result<ScenarioResult> {
    let relay_nodes_count = 3;
    let leaders_per_relay = 10;
    let legacies_per_relay = 1;

    let scenario_settings = prepare_real_scenario(
        "Real-Network",
        relay_nodes_count,
        leaders_per_relay,
        legacies_per_relay,
        &context,
    );
    let mut controller = scenario_settings.build(context)?;

    let core =
        controller.spawn_node(CORE_NODE, LeadershipMode::Leader, PersistenceMode::InMemory)?;

    let mut relays = vec![];
    for i in 0..relay_nodes_count {
        relays.push(controller.spawn_node(
            &relay_name(i + 1),
            LeadershipMode::Leader,
            PersistenceMode::InMemory,
        )?);
    }

    let mut leaders = vec![];
    for i in 0..(relay_nodes_count * leaders_per_relay) {
        leaders.push(controller.spawn_node(
            &leader_name(i + 1),
            LeadershipMode::Leader,
            PersistenceMode::InMemory,
        )?);
    }

    let releases = download_last_n_releases(1);
    let last_release = releases.last().unwrap();
    let legacy_app = get_jormungandr_bin(last_release, controller.working_directory());
    let version = Version::from_str(&last_release.version()).unwrap();

    let mut legacy_leaders = vec![];

    for i in 0..(relay_nodes_count * legacies_per_relay) {
        legacy_leaders.push(
            controller.spawn_legacy_node(
                controller
                    .new_spawn_params(&legacy_name(i + 1))
                    .leadership_mode(LeadershipMode::Leader)
                    .persistence_mode(PersistenceMode::InMemory)
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

    utils::measure_how_many_nodes_are_running(&sync_nodes, "real_network_bootstrap_score");

    let leaders_count = leaders.len() as u64;
    utils::measure_and_log_sync_time(
        &sync_nodes,
        SyncWaitParams::large_network(leaders_count).into(),
        "real_network_sync",
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
        "real_network_single_transaction_propagation",
        MeasurementReportInterval::Standard,
    )?;

    controller.finalize();
    Ok(ScenarioResult::passed())
}
