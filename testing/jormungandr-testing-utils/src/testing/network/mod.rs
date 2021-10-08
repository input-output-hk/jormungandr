pub mod blockchain;
pub mod builder;
pub mod controller;
pub mod rng;
pub mod settings;
pub mod spawn_params;
pub mod topology;
pub mod vote;
pub mod wallet;

pub use blockchain::Blockchain;
use chain_impl_mockchain::header::HeaderId;
pub use rng::{Random, Seed};
use serde::Deserialize;
pub use settings::{NodeSetting, Settings, WalletProxySettings};
pub use spawn_params::{FaketimeConfig, SpawnParams};
use std::path::PathBuf;
pub use topology::{Node, NodeAlias, Topology};
pub use vote::VotePlanKey;
pub use wallet::{ExternalWalletTemplate, Wallet, WalletAlias, WalletTemplate, WalletType};

#[derive(Debug, Copy, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LeadershipMode {
    Leader,
    Passive,
}

#[derive(Debug, Copy, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PersistenceMode {
    Persistent,
    InMemory,
}

#[derive(Debug, Clone)]
pub enum NodeBlock0 {
    Hash(HeaderId),
    File(PathBuf),
}
