use crate::{
    node::{LeadershipMode, PersistenceMode},
    scenario::settings::NodeSetting,
};
use jormungandr_lib::interfaces::{Explorer, Mempool, Policy, TopicsOfInterest};
use std::path::PathBuf;

pub struct SpawnParams {
    pub topics_of_interest: Option<TopicsOfInterest>,
    pub explorer: Option<Explorer>,
    pub mempool: Option<Mempool>,
    pub policy: Option<Policy>,
    pub jormungandr: Option<PathBuf>,
    pub leadership_mode: LeadershipMode,
    pub persistence_mode: PersistenceMode,
    pub alias: String,
}

impl SpawnParams {
    pub fn new(alias: &str) -> Self {
        Self {
            topics_of_interest: None,
            explorer: None,
            mempool: None,
            policy: None,
            jormungandr: None,
            alias: alias.to_owned(),
            leadership_mode: LeadershipMode::Leader,
            persistence_mode: PersistenceMode::Persistent,
        }
    }

    pub fn get_alias(&self) -> String {
        self.alias.clone()
    }

    pub fn get_leadership_mode(&self) -> LeadershipMode {
        self.leadership_mode.clone()
    }

    pub fn get_persistence_mode(&self) -> PersistenceMode {
        self.persistence_mode.clone()
    }

    pub fn topics_of_interest(&mut self, topics_of_interest: TopicsOfInterest) -> &mut Self {
        self.topics_of_interest = Some(topics_of_interest);
        self
    }

    pub fn explorer(&mut self, explorer: Explorer) -> &mut Self {
        self.explorer = Some(explorer);
        self
    }

    pub fn mempool(&mut self, mempool: Mempool) -> &mut Self {
        self.mempool = Some(mempool);
        self
    }

    pub fn policy(&mut self, policy: Policy) -> &mut Self {
        self.policy = Some(policy);
        self
    }

    pub fn jormungandr(&mut self, jormungandr_app_path: PathBuf) -> &mut Self {
        self.jormungandr = Some(jormungandr_app_path);
        self
    }

    pub fn passive(&mut self) -> &mut Self {
        self.leadership_mode = LeadershipMode::Passive;
        self
    }

    pub fn leader(&mut self) -> &mut Self {
        self.leadership_mode = LeadershipMode::Leader;
        self
    }

    pub fn in_memory(&mut self) -> &mut Self {
        self.persistence_mode = PersistenceMode::InMemory;
        self
    }

    pub fn leadership_mode(&mut self, leadership_mode: LeadershipMode) -> &mut Self {
        self.leadership_mode = leadership_mode;
        self
    }

    pub fn persistence_mode(&mut self, persistence_mode: PersistenceMode) -> &mut Self {
        self.persistence_mode = persistence_mode;
        self
    }

    pub fn override_settings(&self, node_settings: &NodeSetting) -> NodeSetting {
        let mut new_settings = node_settings.clone();

        if let Some(topics_of_interest) = &self.topics_of_interest {
            new_settings.config.p2p.topics_of_interest = Some(topics_of_interest.clone());
        }

        if let Some(explorer) = &self.explorer {
            new_settings.config.explorer = explorer.clone();
        }

        if let Some(mempool) = &self.mempool {
            new_settings.config.mempool = Some(mempool.clone());
        }

        if let Some(policy) = &self.policy {
            new_settings.config.p2p.policy = Some(policy.clone());
        }
        new_settings
    }
}
