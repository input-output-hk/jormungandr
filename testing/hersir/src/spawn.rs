use crate::{args::Args, config::Config, error::Error};
use jormungandr_testing_utils::testing::{
    jormungandr::JormungandrProcess,
    network::{builder::NetworkBuilder, NodeAlias},
};
use std::{collections::HashMap, fs::File};

pub fn spawn_network(args: Args) -> Result<HashMap<NodeAlias, JormungandrProcess>, Error> {
    let config: Config = serde_yaml::from_reader(File::open(args.config)?)?;

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

        let spawn_params = config
            .nodes
            .iter()
            .find(|c| c.spawn_params.get_alias() == &alias)
            .map(|c| &c.spawn_params)
            .ok_or_else(|| Error::Internal(format!("Node '{}' has no spawn parameters", alias)))?;

        processes.insert(alias.clone(), controller.spawn(spawn_params.clone())?);

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
