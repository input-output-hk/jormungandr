use crate::{
    args::Args,
    config::Config,
    controller::{MonitorControllerBuilder, MonitorNode},
    error::Error,
};
use jormungandr_automation::jormungandr::NodeAlias;
use std::{collections::HashMap, sync::mpsc::channel};

pub fn spawn_network(config: Config, args: Args) -> Result<(), Error> {
    let mut topology = config.build_topology();
    let (tx, rx) = channel();

    let mut monitor_controller = MonitorControllerBuilder::new(&config.session.title)
        .topology(topology.clone())
        .blockchain(config.build_blockchain())
        .build(config.session.clone())?;

    let mut processes: HashMap<NodeAlias, MonitorNode> = HashMap::new();

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
            monitor_controller.spawn_node_custom(spawn_params.verbose(args.verbose))?,
        );

        topology.nodes.remove(&alias);
        topology.nodes.values_mut().for_each(|n| {
            n.trusted_peers.remove(&alias);
        });
    }

    println!("Waiting for Ctrl-C to exit..");
    monitor_controller.monitor_nodes();

    ctrlc::set_handler(move || {
        for (_, process) in processes.iter_mut() {
            process.finish_monitoring();
        }
        tx.send(()).expect("Could not send signal on channel.")
    })
    .expect("Error setting Ctrl-C handler");

    rx.recv().expect("Could not receive from channel.");
    monitor_controller.finalize();
    Ok(())
}
