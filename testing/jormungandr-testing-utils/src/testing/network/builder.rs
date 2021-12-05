use crate::testing::jormungandr::TestingDirectory;
use crate::testing::utils::{Event, Observable, Observer};
use crate::testing::{
    network::{
        controller::{Controller, ControllerError},
        Blockchain, NodeAlias, NodeSetting, Random, Seed, Settings, Topology, WalletTemplate,
    },
    NodeConfigBuilder,
};
use jormungandr_lib::crypto::key::SigningKey;
use jormungandr_lib::interfaces::NodeSecret;
use std::collections::HashMap;
use std::rc::Rc;
use std::rc::Weak;

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
