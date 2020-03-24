use crate::{
    node::{LeadershipMode, NodeController, PersistenceMode},
    scenario::{
        repository::ScenarioResult, ActiveSlotCoefficient, Blockchain, ConsensusVersion,
        ControllerBuilder, KESUpdateSpeed, Milli, Node, NumberOfSlotsPerEpoch, SlotDuration,
        TopologyBuilder, Value, Wallet,
    },
    test::Result,
    Context,
};

use jormungandr_lib::{interfaces::NodeState, testing::benchmark_efficiency};

use rand_chacha::ChaChaRng;
use std::time::{Duration, SystemTime};

const CORE_NODE: &str = "Core";
const RELAY_NODE: &str = "Relay";
const LEADER_NODE: &str = "Leader";

fn relay_name(i: u32) -> String {
    format!("{}_{}", RELAY_NODE, i)
}

fn leader_name(i: u32) -> String {
    format!("{}_{}", LEADER_NODE, i)
}

fn wallet_name(i: u32) -> String {
    format!("leader_wallet_{}", i)
}

fn prepare_real_scenario(
    title: &str,
    relay_nodes_count: u32,
    nodes_count_per_relay: u32,
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
    for i in 0..relay_nodes_count {
        for _ in 0..nodes_count_per_relay {
            let leader_name = leader_name(leader_counter);
            let mut leader_node = Node::new(&leader_name);

            let relay_name = relay_name(i + 1);
            leader_node.add_trusted_peer(relay_name);
            topology_builder.register_node(leader_node);
            leader_counter = leader_counter + 1;
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
        let mut wallet = Wallet::new_account(initial_wallet_name.to_owned(), Value(100_000));
        *wallet.delegate_mut() = Some(leader_name(i).to_owned());
        blockchain.add_wallet(wallet);
    }
    builder.set_blockchain(blockchain);
    builder.build_settings(&mut context.clone());
    builder
}

pub fn real_network(mut context: Context<ChaChaRng>) -> Result<ScenarioResult> {
    let relay_nodes_count = 8;
    let leaders_per_relay = 10;

    let scenario_settings = prepare_real_scenario(
        "Real-Network",
        relay_nodes_count,
        leaders_per_relay,
        &mut context,
    );
    let mut controller = scenario_settings.build(context.clone())?;

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

    controller.monitor_nodes();
    core.wait_for_bootstrap()?;
    leaders.last().unwrap().wait_for_bootstrap()?;

    measure_how_many_nodes_are_running(&leaders);

    controller.finalize();
    Ok(ScenarioResult::passed())
}

fn measure_how_many_nodes_are_running(leaders: &Vec<NodeController>) {
    let leaders_nodes_count = leaders.len() as u32;

    let mut efficiency_benchmark_run = benchmark_efficiency("real_network_bootstrap_score")
        .target(leaders_nodes_count)
        .start();
    let mut leaders_ids: Vec<u32> = (1..=leaders_nodes_count).collect();
    let now = SystemTime::now();

    loop {
        if now.elapsed().unwrap().as_secs() > (10 * 60) {
            break;
        }
        std::thread::sleep(Duration::from_secs(10));

        leaders_ids.retain(|leader_id| {
            let leader_index_usize = (leader_id - 1) as usize;
            let leader: &NodeController = leaders.get(leader_index_usize).unwrap();
            if let Ok(stats) = leader.stats() {
                if let NodeState::Running = stats.state {
                    efficiency_benchmark_run.increment();
                    return false;
                }
            }
            return true;
        });

        if leaders_ids.is_empty() {
            break;
        }
    }

    print_error_for_failed_leaders(leaders_ids, leaders);

    efficiency_benchmark_run.stop().print()
}

pub fn print_error_for_failed_leaders(leaders_ids: Vec<u32>, leaders: &Vec<NodeController>) {
    if leaders_ids.is_empty() {
        return;
    }

    println!("Nodes which failed to bootstrap: ");
    for leader_id in leaders_ids {
        let leader_index_usize = (leader_id - 1) as usize;
        let error_lines: Vec<String> = leaders
            .get(leader_index_usize)
            .unwrap()
            .logger()
            .get_lines_with_error_and_invalid()
            .collect();
        println!("{} - Error Logs: {:?}", leader_name(leader_id), error_lines);
    }
}
