use crate::{args::Args, config::Config, error::Error};
use jormungandr_testing_utils::testing::{
    jormungandr::JormungandrProcess,
    network::{builder::NetworkBuilder, NodeAlias},
};
use std::{collections::HashMap, fs::File};

pub fn spawn_network(args: Args) -> Result<HashMap<NodeAlias, JormungandrProcess>, Error> {
    let config: Config = serde_yaml::from_reader(File::open(args.config)?)?;

    println!("Building network...");

    let mut topology = config.build_topology();

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

    Ok(processes)
}
