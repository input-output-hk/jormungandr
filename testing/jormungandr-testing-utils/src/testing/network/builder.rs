use crate::testing::{
    network::{
        controller::{Controller, ControllerError},
        wallet::template::builder::WalletTemplateBuilder,
        Blockchain, Node, NodeAlias, NodeSetting, Random, Seed, Settings, TopologyBuilder,
        WalletTemplate,
    },
    NodeConfigBuilder,
};
use chain_impl_mockchain::{chaintypes::ConsensusVersion, milli::Milli};
use jormungandr_lib::crypto::key::SigningKey;
use jormungandr_lib::interfaces::{
    ActiveSlotCoefficient, KesUpdateSpeed, NodeSecret, NumberOfSlotsPerEpoch, SlotDuration,
};

use assert_fs::TempDir;
use std::collections::HashMap;

pub struct NetworkBuilder {
    topology_builder: TopologyBuilder,
    blockchain: Option<Blockchain>,
    wallets: Vec<WalletTemplate>,
}

impl NetworkBuilder {
    pub fn single_trust_direction(&mut self, client: &str, server: &str) -> &mut Self {
        let server_node = Node::new(String::from(server));

        let mut client_node = Node::new(String::from(client));
        client_node.add_trusted_peer(String::from(server));

        self.topology_builder.register_node(server_node);
        self.topology_builder.register_node(client_node);

        self
    }

    pub fn blockchain_config(&mut self, config: Blockchain) -> &mut Self {
        self.blockchain = Some(config);
        self
    }

    pub fn initials(&mut self, wallets: Vec<&mut WalletTemplateBuilder>) -> &mut Self {
        self.wallets.extend(wallets.iter().map(|x| x.build()));
        self
    }

    pub fn build(&self) -> Result<Controller, ControllerError> {
        let temp_dir = TempDir::new().unwrap();
        let topology = self.topology_builder.clone().build();
        let mut blockchain = self.blockchain.clone().unwrap();
        let nodes: HashMap<NodeAlias, NodeSetting> = topology
            .into_iter()
            .map(|(alias, template)| {
                let config = NodeConfigBuilder::new().build();
                (
                    alias.clone(),
                    NodeSetting {
                        alias,
                        config,
                        secret: NodeSecret {
                            bft: None,
                            genesis: None,
                        },
                        topology_secret: SigningKey::generate(&mut rand::thread_rng()),
                        node_topology: template,
                    },
                )
            })
            .collect();
        let seed = Seed::generate(rand::rngs::OsRng);
        let mut random = Random::new(seed);

        for alias in nodes.keys() {
            let leader: NodeAlias = alias.into();
            blockchain.add_leader(leader);
        }

        for wallet in &self.wallets {
            blockchain.add_wallet(wallet.clone());
        }

        let settings = Settings::new(nodes, blockchain, &mut random);
        Controller::new(settings, temp_dir)
    }
}

impl Default for NetworkBuilder {
    fn default() -> Self {
        Self {
            blockchain: Some(Blockchain::new(
                ConsensusVersion::GenesisPraos,
                NumberOfSlotsPerEpoch::new(60).expect("valid number of slots per epoch"),
                SlotDuration::new(2).expect("valid slot duration in seconds"),
                KesUpdateSpeed::new(46800).expect("valid kes update speed in seconds"),
                ActiveSlotCoefficient::new(Milli::from_millis(999))
                    .expect("active slot coefficient in millis"),
            )),
            topology_builder: TopologyBuilder::new(),
            wallets: Vec::new(),
        }
    }
}
