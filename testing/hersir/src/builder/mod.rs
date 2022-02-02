pub mod blockchain;
pub mod rng;
pub mod settings;
pub mod spawn_params;
pub mod topology;
pub mod vote;
mod vote_plan_settings;
pub mod wallet;

use crate::controller::Controller;
pub use crate::controller::Error as ControllerError;
pub use blockchain::Blockchain;
pub use jormungandr_automation::jormungandr::NodeAlias;
use jormungandr_automation::jormungandr::NodeConfigBuilder;
use jormungandr_automation::jormungandr::TestingDirectory;
use jormungandr_automation::testing::observer::{Event, Observable, Observer};
use jormungandr_lib::crypto::key::SigningKey;
use jormungandr_lib::interfaces::NodeSecret;
pub use rng::{Random, Seed};
pub use settings::{NodeSetting, Settings};
pub use spawn_params::SpawnParams;
use std::collections::HashMap;
use std::rc::Rc;
use std::rc::Weak;
pub use topology::{Node, Topology};
pub use vote::VotePlanKey;
pub use vote_plan_settings::VotePlanSettings;
pub use wallet::{ExternalWalletTemplate, Wallet, WalletTemplate, WalletType};

#[derive(Default)]
pub struct NetworkBuilder {
    topology: Topology,
    blockchain: Blockchain,
    wallet_templates: Vec<WalletTemplate>,
    testing_directory: TestingDirectory,
    observers: Vec<Weak<dyn Observer>>,
}

impl Observable for NetworkBuilder {
    fn register(mut self, observer: &Rc<dyn Observer>) -> Self {
        self.observers.push(Rc::downgrade(observer));
        self
    }

    fn notify_all(&self, event: Event) {
        for observer in &self.observers {
            if let Some(observer_listener) = observer.upgrade() {
                observer_listener.notify(event.clone());
            }
        }
    }

    fn finish_all(&self) {
        for observer in &self.observers {
            if let Some(observer_listener) = observer.upgrade() {
                observer_listener.finished();
            }
        }
    }
}

impl NetworkBuilder {
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

    pub fn testing_directory(mut self, testing_directory: TestingDirectory) -> Self {
        self.testing_directory = testing_directory;
        self
    }

    pub fn build(mut self) -> Result<Controller, ControllerError> {
        self.notify_all(Event::new("building topology..."));
        let nodes: HashMap<NodeAlias, NodeSetting> = self
            .topology
            .nodes
            .iter()
            .map(|(alias, node)| {
                let node_config = NodeConfigBuilder::new().build();
                (
                    alias.clone(),
                    NodeSetting {
                        alias: alias.clone(),
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

        self.notify_all(Event::new("building block0.."));
        let settings = Settings::new(nodes, self.blockchain.clone(), &mut random);

        self.finish_all();
        Controller::new(settings, self.testing_directory)
    }
}
