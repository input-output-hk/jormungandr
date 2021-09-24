use std::{collections::HashMap, fs::File};

use jormungandr_testing_utils::testing::{
    jormungandr::JormungandrProcess,
    network::{builder::NetworkBuilder, NodeAlias},
};

use crate::{args::Args, config::Config, error::Error};

pub fn spawn_network(args: Args) -> Result<HashMap<NodeAlias, JormungandrProcess>, Error> {
    let config: Config = serde_json::from_reader(File::open(args.config)?)?;

    println!("{:?}", config);

    let mut topology = config.build_topology();

    let mut controller = NetworkBuilder::default()
        .topology(topology.clone())
        .blockchain_config(config.blockchain)
        .build()?;

    let mut processes: HashMap<NodeAlias, JormungandrProcess> = HashMap::new();

    while !topology.nodes.is_empty() {
        let alias = topology
            .nodes
            .values()
            .find(|n| n.trusted_peers.is_empty())
            .map(|n| n.alias.clone())
            .ok_or(Error::CircularTrust)?;

        processes.insert(alias.clone(), controller.spawn_and_wait(&alias));

        topology.nodes.remove(&alias);
        topology.nodes.values_mut().for_each(|n| {
            n.trusted_peers.remove(&alias);
        });

        if args.verbose {
            println!("Node '{}' started", alias);
        }
    }

    Ok(processes)
}
