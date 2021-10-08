use std::collections::{HashMap, HashSet};

pub type NodeAlias = String;
#[derive(Debug, Clone)]
pub struct Topology {
    pub nodes: HashMap<NodeAlias, Node>,
}

impl Topology {
    pub fn with_node(mut self, node: Node) -> Self {
        self.nodes.insert(node.alias.clone(), node);
        self
    }
}

impl Default for Topology {
    fn default() -> Self {
        Self {
            nodes: HashMap::new(),
        }
    }
}
#[derive(Debug, Clone)]
pub struct Node {
    pub alias: NodeAlias,
    pub trusted_peers: HashSet<NodeAlias>,
}

impl Node {
    pub fn new<S: Into<NodeAlias>>(alias: S) -> Self {
        Self {
            alias: alias.into(),
            trusted_peers: HashSet::new(),
        }
    }

    pub fn with_trusted_peer<S: Into<NodeAlias>>(mut self, peer: S) -> Self {
        self.trusted_peers.insert(peer.into());
        self
    }
}
