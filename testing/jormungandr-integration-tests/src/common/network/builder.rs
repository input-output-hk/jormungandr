use super::{Controller, ControllerError};
use crate::common::{configuration::NodeConfigBuilder, file_utils};
use chain_impl_mockchain::{chaintypes::ConsensusVersion, milli::Milli};
use jormungandr_lib::interfaces::Value;
use jormungandr_lib::interfaces::{
    ActiveSlotCoefficient, KESUpdateSpeed, NodeSecret, NumberOfSlotsPerEpoch, SlotDuration,
};
use jormungandr_testing_utils::testing::network_builder::{
    Blockchain, Node, NodeAlias, NodeSetting, Random, Seed, Settings, SpawnParams, TopologyBuilder,
    WalletAlias, WalletTemplate,
};

use std::collections::HashMap;

pub struct NetworkBuilder {
    title: String,
    topology_builder: TopologyBuilder,
    blockchain: Option<Blockchain>,
    wallets: Vec<WalletTemplate>,
    configs: Vec<SpawnParams>,
}

impl NetworkBuilder {
    pub fn single_trust_direction(&mut self, client: &str, server: &str) -> &mut Self {
        self.star_topology(server, vec![client])
    }

    pub fn star_topology(&mut self, center: &str, satelites: Vec<&str>) -> &mut Self {
        let server_node = Node::new(center.to_string());
        self.topology_builder.register_node(server_node);

        for satelite in satelites {
            let mut satelite_node = Node::new(satelite.to_string());
            satelite_node.add_trusted_peer(center.to_string());
            self.topology_builder.register_node(satelite_node);
        }
        self
    }

    pub fn custom_config(&mut self, spawn_params: Vec<&mut SpawnParams>) -> &mut Self {
        self.configs = spawn_params.iter().map(|x| (**x).clone()).collect();
        self
    }

    pub fn initials(&mut self, wallets: Vec<&mut WalletTemplateBuilder>) -> &mut Self {
        self.wallets.extend(wallets.iter().map(|x| x.build()));
        self
    }

    pub fn build(&self) -> Result<Controller, ControllerError> {
        let topology = self.topology_builder.clone().build();
        let mut blockchain = self.blockchain.clone().unwrap();
        let nodes: HashMap<NodeAlias, NodeSetting> = topology
            .into_iter()
            .map(|(alias, template)| {
                let mut config = NodeConfigBuilder::new().build();
                if let Some(spawn_params) =
                    self.configs.clone().iter().find(|x| x.get_alias() == alias)
                {
                    spawn_params.override_settings(&mut config);
                }

                (
                    alias.clone(),
                    NodeSetting {
                        alias,
                        config: config,
                        secret: NodeSecret {
                            bft: None,
                            genesis: None,
                        },
                        node_topology: template,
                    },
                )
            })
            .collect();
        let seed = Seed::generate(rand::rngs::OsRng);
        let mut random = Random::new(seed);

        for (alias, _) in &nodes {
            let leader: NodeAlias = alias.into();
            blockchain.add_leader(leader);
        }

        for wallet in &self.wallets {
            blockchain.add_wallet(wallet.clone());
        }

        let settings = Settings::new(nodes, blockchain, &mut random);
        Controller::new(
            &self.title,
            settings,
            file_utils::get_path_in_temp(&self.title),
        )
    }
}

pub fn builder(title: &str) -> NetworkBuilder {
    NetworkBuilder {
        title: title.to_string(),
        blockchain: Some(Blockchain::new(
            ConsensusVersion::GenesisPraos,
            NumberOfSlotsPerEpoch::new(60).expect("valid number of slots per epoch"),
            SlotDuration::new(2).expect("valid slot duration in seconds"),
            KESUpdateSpeed::new(46800).expect("valid kes update speed in seconds"),
            ActiveSlotCoefficient::new(Milli::from_millis(999))
                .expect("active slot coefficient in millis"),
        )),
        topology_builder: TopologyBuilder::new(),
        wallets: Vec::new(),
        configs: Vec::new(),
    }
}

pub struct WalletTemplateBuilder {
    alias: WalletAlias,
    value: Value,
    wallet_template: Option<WalletTemplate>,
    node_alias: Option<NodeAlias>,
}

impl WalletTemplateBuilder {
    pub fn with(&mut self, value: u64) -> &mut Self {
        self.value = value.into();
        self
    }

    pub fn delegated_to(&mut self, delegated_to: &str) -> &mut Self {
        self.node_alias = Some(delegated_to.to_string());
        self
    }

    pub fn build(&self) -> WalletTemplate {
        let mut wallet = WalletTemplate::new_account(self.alias.clone(), self.value);
        *wallet.delegate_mut() = self.node_alias.clone();
        wallet
    }
}

pub fn wallet(alias: &str) -> WalletTemplateBuilder {
    WalletTemplateBuilder {
        alias: alias.to_string(),
        value: 0u64.into(),
        wallet_template: None,
        node_alias: None,
    }
}

pub fn params(alias: &str) -> SpawnParams {
    SpawnParams::new(&alias)
}
