use std::collections::HashMap;

pub type NodeAlias = String;
#[derive(Debug, Clone)]
pub struct Topology {
    nodes: HashMap<NodeAlias, Node>,
}

impl Topology {
    pub fn with_node(mut self, node: Node) -> Self {
        self.nodes.insert(node.alias().clone(), node);
        self
    }

    pub fn nodes(&self) -> impl Iterator<Item = &Node> {
        self.nodes.values()
    }
}

impl Default for Topology {
    fn default() -> Self {
        Self {
            nodes: HashMap::new(),
        }
    }
}

impl IntoIterator for Topology {
    type Item = (NodeAlias, Node);
    type IntoIter = std::collections::hash_map::IntoIter<NodeAlias, Node>;

    fn into_iter(self) -> Self::IntoIter {
        self.nodes.into_iter()
    }
}

#[derive(Debug, Clone)]
pub struct Node {
    alias: NodeAlias,
    trusted_peers: Vec<NodeAlias>,
}

impl Node {
    pub fn new<S: Into<NodeAlias>>(alias: S) -> Self {
        Self {
            alias: alias.into(),
            trusted_peers: Vec::new(),
        }
    }

    pub fn alias(&self) -> &NodeAlias {
        &self.alias
    }

    pub fn with_trusted_peer<S: Into<NodeAlias>>(mut self, peer: S) -> Self {
        self.trusted_peers.push(peer.into());
        self
    }

    pub fn trusted_peers(&self) -> impl Iterator<Item = &NodeAlias> {
        self.trusted_peers.iter()
    }
}
