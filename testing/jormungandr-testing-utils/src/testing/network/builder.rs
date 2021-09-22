use crate::testing::{
    network::{
        controller::{Controller, ControllerError},
        Blockchain, Node, NodeAlias, NodeSetting, Random, Seed, Settings, Topology, WalletTemplate,
    },
    NodeConfigBuilder,
};
use assert_fs::TempDir;
use jormungandr_lib::crypto::key::SigningKey;
use jormungandr_lib::interfaces::NodeSecret;
use std::collections::HashMap;

#[derive(Default)]
pub struct NetworkBuilder {
    topology: Topology,
    blockchain: Blockchain,
    wallet_templates: Vec<WalletTemplate>,
}

impl NetworkBuilder {
    pub fn single_trust_direction(mut self, client: &str, server: &str) -> Self {
        self.topology = Topology::default()
            .with_node(Node::new(server))
            .with_node(Node::new(client).with_trusted_peer(server));
        self
    }

    pub fn topology(mut self, topology: Topology) -> Self {
        self.topology = topology;
        self
    }

    pub fn blockchain_config(mut self, config: Blockchain) -> Self {
        self.blockchain = config;
        self
    }

    pub fn wallet_template(mut self, wallet: WalletTemplate) -> Self {
        self.wallet_templates.push(wallet);
        self
    }

    pub fn build(mut self) -> Result<Controller, ControllerError> {
        let temp_dir = TempDir::new().unwrap();
        let nodes: HashMap<NodeAlias, NodeSetting> = self
            .topology
            .nodes()
            .map(|node| {
                let node_config = NodeConfigBuilder::new().build();
                (
                    node.alias().clone(),
                    NodeSetting {
                        alias: node.alias().clone(),
                        config: node_config,
                        secret: NodeSecret {
                            bft: None,
                            genesis: None,
                        },
                        topology_secret: SigningKey::generate(&mut rand::thread_rng()),
                        node_topology: node.clone(),
                    },
                )
            })
            .collect();
        let seed = Seed::generate(rand::rngs::OsRng);
        let mut random = Random::new(seed);

        for alias in nodes.keys() {
            let leader: NodeAlias = alias.into();
            self.blockchain.add_leader(leader);
        }

        for wallet in &self.wallet_templates {
            self.blockchain.add_wallet(wallet.clone());
        }

        let settings = Settings::new(nodes, self.blockchain, &mut random);
        Controller::new(settings, temp_dir)
    }
}
