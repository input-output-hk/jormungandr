use crate::testing::{
    network::{
        controller::{Controller, ControllerError},
        wallet::template::builder::WalletTemplateBuilder,
        Blockchain, Node, NodeAlias, NodeSetting, Random, Seed, Settings, Topology, WalletTemplate,
    },
    NodeConfigBuilder,
};
use assert_fs::TempDir;
use chain_impl_mockchain::{chaintypes::ConsensusVersion, milli::Milli};
use jormungandr_lib::crypto::key::SigningKey;
use jormungandr_lib::interfaces::{
    ActiveSlotCoefficient, KesUpdateSpeed, NodeSecret, NumberOfSlotsPerEpoch, SlotDuration,
};
use std::collections::HashMap;

pub struct NetworkBuilder {
    topology: Topology,
    blockchain: Blockchain,
    wallets: Vec<WalletTemplate>,
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

    pub fn initials(mut self, wallets: Vec<&mut WalletTemplateBuilder>) -> Self {
        self.wallets.extend(wallets.iter().map(|x| x.build()));
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

        for wallet in &self.wallets {
            self.blockchain.add_wallet(wallet.clone());
        }

        let settings = Settings::new(nodes, self.blockchain, &mut random);
        Controller::new(settings, temp_dir)
    }
}

impl Default for NetworkBuilder {
    fn default() -> Self {
        Self {
            blockchain: Blockchain::new(
                ConsensusVersion::GenesisPraos,
                NumberOfSlotsPerEpoch::new(60).expect("valid number of slots per epoch"),
                SlotDuration::new(2).expect("valid slot duration in seconds"),
                KesUpdateSpeed::new(46800).expect("valid kes update speed in seconds"),
                ActiveSlotCoefficient::new(Milli::from_millis(999))
                    .expect("active slot coefficient in millis"),
            ),
            topology: Topology::default(),
            wallets: Vec::new(),
        }
    }
}
