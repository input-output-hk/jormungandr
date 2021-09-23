use std::{collections::HashMap, fs::File};

use jormungandr_testing_utils::testing::{
    jormungandr::JormungandrProcess,
    network::{builder::NetworkBuilder, NodeAlias},
};

use crate::{args::Args, config::Config, error::Error};

pub fn spawn_network(args: Args) -> Result<HashMap<NodeAlias, JormungandrProcess>, Error> {
    let config: Config = serde_json::from_reader(File::open(args.config).unwrap()).unwrap();

    let mut topology = config.build_topology();

    let mut controller = NetworkBuilder::default()
        .topology(topology.clone())
        .build()
        .unwrap();

    let mut processes: HashMap<NodeAlias, JormungandrProcess> = HashMap::new();

    while !topology.nodes.is_empty() {
        if let Some(alias) = topology
            .nodes
            .values()
            .find(|n| n.trusted_peers.is_empty())
            .map(|n| n.alias.clone())
        {
            processes.insert(alias.clone(), controller.spawn_and_wait(&alias));

            topology.nodes.remove(&alias);
            topology.nodes.iter_mut().for_each(|(_, n)| {
                n.trusted_peers.remove(&alias);
            });
        } else {
            panic!("Circular trust dependency found in network topology");
        }
    }

    Ok(processes)
}
