use super::indexing::ExplorerAddress;
use super::{EpochData, ExplorerBlock};
use crate::blockcfg::{ChainLength, Epoch, FragmentId, HeaderHash};
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use tracing::debug;

#[derive(Error, Debug)]
pub enum StableIndexError {
    #[error("block is already indexed in stable explorer storage: {0}")]
    BlockAlreadyExists(HeaderHash),
    #[error("there is already a block with given chain_length: {0}")]
    DuplicatedChainLength(ChainLength),
    #[error("transaction is already indexed in stable explorer storage")]
    TransactionAlreadyExists,
}

#[derive(Clone, Default)]
pub struct StableIndexShared(pub Arc<RwLock<StableIndex>>);

impl StableIndexShared {
    pub async fn write<'a>(&'a self) -> RwLockWriteGuard<'a, StableIndex> {
        self.0.write().await
    }

    pub async fn read<'a>(&'a self) -> RwLockReadGuard<'a, StableIndex> {
        self.0.read().await
    }
}

/// in memory non-versioned version of the explorer indexes
/// this is mostly a *naive* version, because the final step would be to have
/// something backed by an on-disk database
/// ideally just reimplementing this would be enough to introduce a database
/// in practice, the api may need to be adapted to use database cursors or some
/// sort of offsets
#[derive(Default)]
pub struct StableIndex {
    transactions_by_address: HashMap<ExplorerAddress, Vec<FragmentId>>,
    block_by_chain_length: BTreeMap<ChainLength, HeaderHash>,
    epochs: BTreeMap<Epoch, EpochData>,
    blocks: HashMap<HeaderHash, ExplorerBlock>,
    transaction_to_block: HashMap<FragmentId, HeaderHash>,
}

impl StableIndex {
    pub fn apply_block(&mut self, block: ExplorerBlock) -> Result<(), StableIndexError> {
        debug!("applying block to explorer's stable index {}", block.id());

        if self
            .block_by_chain_length
            .insert(block.chain_length, block.id())
            .is_some()
        {
            return Err(StableIndexError::DuplicatedChainLength(block.chain_length));
        }

        for (hash, tx) in &block.transactions {
            let included_addresses: std::collections::HashSet<ExplorerAddress> = tx
                .outputs()
                .iter()
                .map(|output| output.address.clone())
                .chain(tx.inputs().iter().map(|input| input.address.clone()))
                .collect();

            for address in included_addresses {
                self.transactions_by_address
                    .entry(address)
                    .or_insert(vec![])
                    .push(*hash)
            }

            if self.transaction_to_block.insert(*hash, block.id).is_some() {
                return Err(StableIndexError::TransactionAlreadyExists);
            }
        }

        self.epochs
            .entry(block.date.epoch)
            .and_modify(|epoch_data| {
                epoch_data.last_block = block.id;
                epoch_data.total_blocks += 1;
            })
            .or_insert(EpochData {
                first_block: block.id,
                last_block: block.id,
                total_blocks: 1,
            });

        let id = block.id.clone();

        if self.blocks.insert(block.id, block).is_some() {
            return Err(StableIndexError::BlockAlreadyExists(id));
        }

        Ok(())
    }

    pub fn last_block_length(&self) -> Option<ChainLength> {
        self.block_by_chain_length
            .keys()
            .last()
            .map(ChainLength::clone)
    }

    pub fn get_block(&self, block_id: &HeaderHash) -> Option<&ExplorerBlock> {
        self.blocks.get(block_id)
    }

    pub fn transactions_by_address(
        &self,
        address: &ExplorerAddress,
    ) -> Option<impl Iterator<Item = &FragmentId>> {
        self.transactions_by_address
            .get(address)
            .map(|inner| inner.iter())
    }

    pub fn get_block_by_chain_length(&self, chain_length: &ChainLength) -> Option<&HeaderHash> {
        self.block_by_chain_length.get(chain_length)
    }

    pub fn get_epoch_data(&self, epoch: &Epoch) -> Option<&EpochData> {
        self.epochs.get(epoch)
    }

    pub fn transaction_to_block(&self, fragment_id: &FragmentId) -> Option<&HeaderHash> {
        self.transaction_to_block.get(fragment_id)
    }

    pub fn get_block_hash_range(
        &self,
        from: ChainLength,
        to: ChainLength,
    ) -> impl Iterator<Item = (HeaderHash, ChainLength)> + '_ {
        self.block_by_chain_length
            .range(from..to)
            .map(|(length, hash)| (hash.clone(), length.clone()))
    }
}
