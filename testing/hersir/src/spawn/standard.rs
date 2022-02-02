use crate::builder::NetworkBuilder;
use crate::{args::Args, builder::Topology, config::Config, error::Error};
use jormungandr_automation::jormungandr::JormungandrProcess;
use jormungandr_automation::jormungandr::NodeAlias;
use std::collections::HashMap;
use std::time::Duration;

pub fn spawn_network(config: Config, mut topology: Topology, args: Args) -> Result<(), Error> {
    println!("Building network...");
    let mut controller = NetworkBuilder::default()
        .topology(topology.clone())
        .testing_directory(config.testing_directory())
        .blockchain_config(config.blockchain.clone())
        .build()?;

    let mut processes: HashMap<NodeAlias, JormungandrProcess> = HashMap::new();

    while !topology.nodes.is_empty() {
        let alias = topology
            .nodes
            .values()
            .find(|n| n.trusted_peers.is_empty())
            .map(|n| n.alias.clone())
            .ok_or(Error::CircularTrust)?;

        let spawn_params = config.node_spawn_params(&alias)?;

        processes.insert(
            alias.clone(),
            controller.spawn(spawn_params.verbose(args.verbose))?,
        );

        topology.nodes.remove(&alias);
        topology.nodes.values_mut().for_each(|n| {
            n.trusted_peers.remove(&alias);
        });

        println!("Node '{}' started", alias);
    }

    println!("Network is started");
    loop {
        for node in processes.values() {
            let _result = node.rest().network_stats()?;
        }
        std::thread::sleep(Duration::from_secs(1));
    }
}
