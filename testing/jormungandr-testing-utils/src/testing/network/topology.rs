use std::{borrow::Borrow, collections::HashMap, hash::Hash};

pub type NodeAlias = String;

#[derive(Debug, Clone)]
pub struct Node {
    alias: NodeAlias,

    trusted_peers: Vec<NodeAlias>,
}

#[derive(Debug, Clone)]
pub struct Topology {
    nodes: HashMap<NodeAlias, Node>,
}

#[derive(Debug, Clone)]
pub struct TopologyBuilder {
    nodes: HashMap<NodeAlias, Node>,
}

impl Node {
    pub fn new<S: Into<NodeAlias>>(alias: S) -> Self {
        Node {
            alias: alias.into(),
            trusted_peers: Vec::new(),
        }
    }

    pub fn alias(&self) -> &NodeAlias {
        &self.alias
    }

    pub fn add_trusted_peer<S: Into<NodeAlias>>(&mut self, peer: S) {
        self.trusted_peers.push(peer.into())
    }

    pub fn trusted_peers(&self) -> impl Iterator<Item = &NodeAlias> {
        self.trusted_peers.iter()
    }
}

impl IntoIterator for Topology {
    type Item = (NodeAlias, Node);
    type IntoIter = std::collections::hash_map::IntoIter<NodeAlias, Node>;

    fn into_iter(self) -> Self::IntoIter {
        self.nodes.into_iter()
    }
}

impl Topology {
    pub fn node<K>(&self, alias: &K) -> Option<&Node>
    where
        NodeAlias: Borrow<K>,
        K: Hash + Eq,
    {
        self.nodes.get(alias)
    }

    pub fn aliases(&self) -> impl Iterator<Item = &NodeAlias> {
        self.nodes.keys()
    }

    pub fn format_into_graphviz_dot<W: std::io::Write>(&self, mut writer: W) -> std::io::Result<W> {
        writeln!(writer, "digraph NodeTopology {{")?;

        for node in self.nodes.values() {
            for edge in node.trusted_peers() {
                writeln!(writer, "  {} -> {}", node.alias(), edge)?;
            }
        }

        writeln!(writer, "}}")?;

        Ok(writer)
    }
}

impl TopologyBuilder {
    pub fn new() -> Self {
        TopologyBuilder {
            nodes: HashMap::new(),
        }
    }

    pub fn register_node(&mut self, node: Node) {
        self.nodes.insert(node.alias().clone(), node);
    }

    pub fn build(self) -> Topology {
        for node in self.nodes.values() {
            for trusted_peer in node.trusted_peers() {
                if !self.nodes.contains_key(trusted_peer) {
                    panic!("Trusted peer has not been defined")
                }
            }
        }

        Topology { nodes: self.nodes }
    }
}

impl Default for TopologyBuilder {
    fn default() -> Self {
        Self::new()
    }
}
