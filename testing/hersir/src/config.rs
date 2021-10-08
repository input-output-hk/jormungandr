use jormungandr_testing_utils::testing::network::{
    Blockchain, Node, NodeAlias, SpawnParams, Topology,
};
use serde::Deserialize;
use std::collections::HashSet;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub blockchain: Blockchain,
    pub nodes: Vec<NodeConfig>,
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
pub struct NodeConfig {
    pub spawn_params: SpawnParams,
    #[serde(default)]
    pub trusted_peers: HashSet<NodeAlias>,
}
