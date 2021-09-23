use jormungandr_lib::interfaces::Block0Configuration;
use jormungandr_testing_utils::testing::network::{Node, NodeAlias, SpawnParams, Topology};
use serde::Deserialize;
use std::collections::HashSet;

#[derive(Debug, Deserialize)]
pub struct Config {
    blockchain: Option<Block0Configuration>,
    nodes: Vec<NodeConfig>,
}

impl Config {
    pub fn build_topology(&self) -> Topology {
        let mut topology = Topology::default();

        for node_config in self.nodes.iter() {
            let mut node = Node::new(node_config.spawn_params.get_alias());

            for trusted_peer in node_config.trusted_peers.iter() {
                node = node.with_trusted_peer(trusted_peer);
            }

            topology = topology.with_node(node);
        }

        topology
    }
}

#[derive(Debug, Deserialize)]
struct NodeConfig {
    spawn_params: SpawnParams,
    #[serde(default)]
    trusted_peers: HashSet<NodeAlias>,
}
