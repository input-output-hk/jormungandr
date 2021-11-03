use crate::testing::network::NodeAlias;
use jormungandr_lib::crypto::hash::Hash;
use std::{fmt, time::Duration};
use thiserror::Error;

pub trait SyncNode {
    fn alias(&self) -> NodeAlias;
    fn last_block_height(&self) -> u32;
    fn log_stats(&self);
    fn tip(&self) -> Hash;
    fn log_content(&self) -> String;
    fn get_lines_with_error_and_invalid(&self) -> Vec<String>;
    fn is_running(&self) -> bool;
}

#[derive(Debug, Clone)]
pub struct SyncNodeRecord {
    pub alias: String,
    pub block_height: u32,
}

impl fmt::Display for SyncNodeRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({} -> {})", self.alias, self.block_height)
    }
}

impl SyncNodeRecord {
    pub fn new(alias: String, block_height: u32) -> Self {
        Self {
            alias,
            block_height,
        }
    }
}

#[derive(Debug, Error)]
pub enum SyncNodeError {
    #[error(
        "Timeout exceeded '{timeout:?}'. Target node: {target_node}. Sync nodes: {sync_nodes:?}"
    )]
    TimeoutWhenSyncingTargetNode {
        timeout: Duration,
        target_node: SyncNodeRecord,
        sync_nodes: Vec<SyncNodeRecord>,
    },
    #[error("verification error")]
    VerificationError(#[from] crate::testing::verify::Error),
}
