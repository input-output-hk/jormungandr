use crate::builder::settings::NodeTemplate;
use chain_crypto::Ed25519;
use jormungandr_automation::jormungandr::NodeAlias;
use jormungandr_lib::{
    crypto::key::SigningKey,
    interfaces::{NodeConfig, NodeSecret},
};

/// contains all the data to start or interact with a node
#[derive(Debug, Clone)]
pub struct NodeSetting {
    /// for reference purpose only
    pub alias: NodeAlias,

    /// node secret, this will be passed to the node at start
    /// up of the node. It may contains the necessary crypto
    /// for the node to be a blockchain leader (BFT leader or
    /// stake pool)
    pub secret: NodeSecret,

    pub config: NodeConfig,

    pub topology_secret: SigningKey<Ed25519>,

    pub node_topology: NodeTemplate,
}
