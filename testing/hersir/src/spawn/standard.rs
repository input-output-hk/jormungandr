use crate::{args::Args, builder::NetworkBuilder, config::Config, error::Error};
use jormungandr_automation::jormungandr::{JormungandrProcess, NodeAlias};
use std::{collections::HashMap, time::Duration};

pub fn spawn_network(config: Config, args: Args) -> Result<(), Error> {
    let mut topology = config.build_topology();

    println!("Building network...");
    let mut controller = NetworkBuilder::default()
        .apply_config(config.clone())
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

    let _maybe_explorer = {
        if controller.settings().explorer.is_some() {
            let explorer = Some(controller.spawn_explorer()?);
            println!("explorer started");
            explorer
        } else {
            None
        }
    };

    println!("Network is started");
    loop {
        for node in processes.values() {
            let _result = node.rest().network_stats()?;
        }
        std::thread::sleep(Duration::from_secs(1));
    }
}
