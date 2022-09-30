mod committee;
mod explorer;
pub mod rng;
pub mod settings;
mod stake_pool;
pub mod topology;
pub mod vote;
pub mod wallet;

pub use crate::controller::Error as ControllerError;
use crate::{
    config::{
        Blockchain, CommitteeTemplate, Config, ExplorerTemplate, SessionSettings, VotePlanTemplate,
        WalletTemplate,
    },
    controller::{Controller, Error},
    utils::Dotifier,
};
pub use jormungandr_automation::jormungandr::NodeAlias;
use jormungandr_automation::{
    jormungandr::NodeConfigBuilder,
    testing::observer::{Event, Observable, Observer},
};
use jormungandr_lib::{crypto::key::SigningKey, interfaces::NodeSecret};
pub use rng::{Random, Seed};
pub use settings::{vote_plan::VotePlanSettings, wallet::Wallet, NodeSetting, Settings};
use std::{
    collections::HashMap,
    path::Path,
    rc::{Rc, Weak},
};
pub use topology::{Node, Topology};
pub use vote::VotePlanKey;

#[derive(Default)]
pub struct NetworkBuilder {
    topology: Topology,
    blockchain: Blockchain,
    session_settings: SessionSettings,
    explorer_template: Option<ExplorerTemplate>,
    wallet_templates: Vec<WalletTemplate>,
    committee_templates: Vec<CommitteeTemplate>,
    vote_plan_templates: Vec<VotePlanTemplate>,
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
    pub fn apply_config(self, config: Config) -> Self {
        self.topology(config.build_topology())
            .blockchain_config(config.build_blockchain())
            .session_settings(config.session)
            .wallet_templates(config.wallets)
            .vote_plan_templates(config.vote_plans)
            .committees(config.committees)
            .explorer(config.explorer)
    }

    pub fn topology(mut self, topology: Topology) -> Self {
        self.topology = topology;
        self
    }

    pub fn blockchain_config(mut self, config: Blockchain) -> Self {
        self.blockchain = config;
        self
    }

    pub fn wallet_templates(mut self, wallets: Vec<WalletTemplate>) -> Self {
        self.wallet_templates = wallets;
        self
    }

    pub fn wallet_template(mut self, wallet: WalletTemplate) -> Self {
        self.wallet_templates.push(wallet);
        self
    }

    pub fn vote_plan_templates(mut self, vote_plans: Vec<VotePlanTemplate>) -> Self {
        self.vote_plan_templates = vote_plans;
        self
    }

    pub fn committees(mut self, committee_templates: Vec<CommitteeTemplate>) -> Self {
        self.committee_templates = committee_templates;
        self
    }

    pub fn session_settings(mut self, session_settings: SessionSettings) -> Self {
        self.session_settings = session_settings;
        self
    }

    pub fn build(self) -> Result<Controller, ControllerError> {
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

        self.notify_all(Event::new("building block0.."));
        let settings = Settings::new(
            nodes,
            &self.blockchain,
            &self.wallet_templates,
            &self.committee_templates,
            &self.explorer_template,
            &self.vote_plan_templates,
            &mut random,
        )?;

        self.notify_all(Event::new("dumping wallet secret keys.."));

        if self.session_settings.generate_documentation {
            document(self.session_settings.root.path(), &settings)?;
        }

        self.finish_all();
        Controller::new(settings, self.session_settings.root)
    }

    pub fn explorer(mut self, explorer: Option<ExplorerTemplate>) -> Self {
        self.explorer_template = explorer;
        self
    }
}

fn document(path: &Path, settings: &Settings) -> Result<(), Error> {
    let file = std::fs::File::create(&path.join("initial_setup.dot"))?;

    let dotifier = Dotifier;
    dotifier.dottify(settings, file)?;

    for wallet in &settings.wallets {
        wallet.save_to(path)?;
    }

    let file = std::fs::File::create(&path.join("genesis.yaml"))?;
    serde_yaml::to_writer(file, &settings.block0).unwrap();

    Ok(())
}
