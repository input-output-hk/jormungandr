mod blockchain;
mod rng;
mod settings;
mod spawn_params;
mod topology;
mod wallet;

pub use blockchain::Blockchain;
use chain_impl_mockchain::header::HeaderId;
pub use rng::{Random, Seed};
pub use settings::{NodeSetting, Settings};
pub use spawn_params::SpawnParams;
use std::path::PathBuf;
pub use topology::{Node, NodeAlias, Topology, TopologyBuilder};
pub use wallet::{Wallet, WalletAlias, WalletTemplate, WalletType};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum LeadershipMode {
    Leader,
    Passive,
}

#[derive(Debug, Clone)]
pub enum NodeBlock0 {
    Hash(HeaderId),
    File(PathBuf),
}
