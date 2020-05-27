use chain_impl_mockchain::key::Hash;
use std::{fmt, time::Duration};
use thiserror::Error;

use crate::testing::measurement::{benchmark_speed, Speed, Thresholds};

mod wait;
pub use wait::SyncWaitParams;

pub trait SyncNode {
    fn alias(&self) -> &str;
    fn last_block_height(&self) -> u32;
    fn log_stats(&self);
    fn all_blocks_hashes(&self) -> Vec<Hash>;
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
}

pub fn assure_node_in_sync(
    target_node: &dyn SyncNode,
    other_nodes: Vec<&dyn SyncNode>,
    sync_wait: Thresholds<Speed>,
    info: &str,
) -> Result<(), SyncNodeError> {
    let benchmark = benchmark_speed(info.to_owned())
        .with_thresholds(sync_wait)
        .start();

    while !benchmark.timeout_exceeded() {
        let target_node_block_height = target_node.last_block_height();

        let max_block_height: u32 = other_nodes
            .iter()
            .map(|node| node.last_block_height())
            .max()
            .expect("unable to retrieve block height from sync nodes");

        if target_node_block_height >= max_block_height {
            benchmark.stop();
            return Ok(());
        }
    }

    let other_nodes_records: Vec<SyncNodeRecord> = other_nodes
        .iter()
        .map(|x| SyncNodeRecord::new(x.alias().to_string(), x.last_block_height()))
        .collect();

    let target_node = SyncNodeRecord::new(
        target_node.alias().to_string(),
        target_node.last_block_height(),
    );

    Err(SyncNodeError::TimeoutWhenSyncingTargetNode {
        target_node: target_node,
        sync_nodes: other_nodes_records,
        timeout: benchmark.definition().thresholds().unwrap().max().into(),
    })
}
