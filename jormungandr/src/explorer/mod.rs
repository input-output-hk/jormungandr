pub mod error;
pub mod graphql;
mod indexing;
mod multiverse;
mod persistent_sequence;
mod stable_storage;

use self::error::{ExplorerError as Error, Result};
use self::graphql::EContext;
use self::indexing::{
    Addresses, Blocks, ChainLengths, EpochData, Epochs, ExplorerAddress, ExplorerBlock,
    ExplorerVotePlan, ExplorerVoteProposal, ExplorerVoteTally, StakePool, StakePoolBlocks,
    StakePoolData, Transactions, VotePlans,
};
use self::persistent_sequence::PersistentSequence;
use tracing::{debug, span, Level};
use tracing_futures::Instrument;

use crate::blockcfg::{
    Block, ChainLength, ConfigParam, ConfigParams, ConsensusVersion, Epoch, Fragment, FragmentId,
    HeaderHash,
};
use crate::blockchain::{self, Blockchain, MAIN_BRANCH_TAG};
use crate::explorer::indexing::ExplorerVote;
use crate::intercom::ExplorerMsg;
use crate::utils::async_msg::MessageQueue;
use crate::utils::task::TokioServiceInfo;
use chain_addr::Discrimination;
use chain_core::property::Block as _;
use chain_impl_mockchain::certificate::{Certificate, PoolId, VotePlanId};
use chain_impl_mockchain::fee::LinearFee;
use futures::prelude::*;
use multiverse::Multiverse;
use stable_storage::StableIndexShared;
use std::collections::VecDeque;
use std::convert::Infallible;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex, RwLock};

#[derive(Clone)]
pub struct Explorer {
    pub db: ExplorerDb,
}

#[derive(Clone)]
struct Tip(Arc<RwLock<HeaderHash>>);

#[derive(Clone)]
pub struct ExplorerDb {
    /// Structure that keeps all the known states to allow easy branch management
    /// each new block is indexed by getting its previous `State` from the multiverse
    /// and inserted a new updated one.
    multiverse: Multiverse,
    /// This keeps track of the longest chain seen until now. All the queries are
    /// performed using the state of this branch, the HeaderHash is used as key for the
    /// multiverse, and the ChainLength is used in the updating process.
    longest_chain_tip: Tip,
    pub blockchain_config: BlockchainConfig,
    blockchain: Blockchain,
    blockchain_tip: blockchain::Tip,
    tip_broadcast: tokio::sync::broadcast::Sender<(HeaderHash, multiverse::Ref)>,
    stable_storage: StableIndexShared,
}

#[derive(Clone)]
pub struct BlockchainConfig {
    /// Used to construct `Address` from `AccountIndentifier` when processing transaction
    /// inputs
    discrimination: Discrimination,
    consensus_version: ConsensusVersion,
    fees: LinearFee,
    epoch_stability_depth: u32,
}

/// Inmutable data structure used to represent the explorer's state at a given Block
/// A new state can be obtained to from a Block and it's previous state, getting two
/// independent states but with memory sharing to minimize resource utilization
#[derive(Clone)]
pub(self) struct State {
    pub transactions: Transactions,
    pub blocks: Blocks,
    addresses: Addresses,
    epochs: Epochs,
    chain_lengths: ChainLengths,
    stake_pool_data: StakePool,
    stake_pool_blocks: StakePoolBlocks,
    vote_plans: VotePlans,
}

#[derive(Clone)]
pub struct Settings {
    /// This is the prefix that's used for the Address bech32 string representation in the
    /// responses (in the queries any prefix can be used). base32 serialization could
    /// also be used, but the `Address` struct doesn't have a deserialization method right
    /// now
    pub address_bech32_prefix: String,
}

impl Explorer {
    pub fn new(db: ExplorerDb) -> Explorer {
        Explorer { db }
    }

    pub fn context(&self) -> EContext {
        EContext {
            db: self.db.clone(),
            settings: Settings {
                // Hardcoded bech32 prefix
                address_bech32_prefix: "addr".to_owned(),
            },
        }
    }

    pub async fn start(&self, info: TokioServiceInfo, messages: MessageQueue<ExplorerMsg>) {
        let tip_candidate: Arc<Mutex<Option<HeaderHash>>> = Arc::new(Mutex::new(None));
        let span_parent = info.span();
        messages
            .for_each(|input| {
                let explorer_db = self.db.clone();
                let tip_candidate = Arc::clone(&tip_candidate);
                match input {
                    ExplorerMsg::NewBlock(block) => {
                        info.spawn_fallible::<_, Error>(
                            "apply block to explorer",
                            async move {
                                let _state_ref = explorer_db.apply_block(block.clone()).await?;

                                let mut guard = tip_candidate.lock().await;
                                if guard.map(|hash| hash == block.header.id()).unwrap_or(false) {
                                    let hash = guard.take().unwrap();
                                    explorer_db.set_tip(hash).await?;
                                }

                                Ok(())
                            }
                            .instrument(span!(
                                parent: span_parent,
                                Level::TRACE,
                                "apply block",
                            )),
                        );
                    }
                    ExplorerMsg::NewTip(hash) => {
                        info.spawn_fallible::<_, Error>(
                            "apply tip to explorer",
                            async move {
                                let successful = explorer_db.set_tip(hash).await?;

                                if !successful {
                                    let mut guard = tip_candidate.lock().await;
                                    guard.replace(hash);
                                }

                                Ok(())
                            }
                            .instrument(span!(
                                parent: span_parent,
                                Level::TRACE,
                                "apply tip",
                            )),
                        );
                    }
                };

                futures::future::ready(())
            })
            .await;
    }
}

impl ExplorerDb {
    /// Apply all the blocks in the [block0, MAIN_BRANCH_TAG], also extract the static
    /// Blockchain settings from the Block0 (Discrimination)
    /// This function is only called once on the node's bootstrap phase
    pub async fn bootstrap(
        block0: Block,
        blockchain: &Blockchain,
        blockchain_tip: blockchain::Tip,
    ) -> Result<Self> {
        let blockchain_config = BlockchainConfig::from_config_params(
            block0
                .contents
                .iter()
                .filter_map(|fragment| match fragment {
                    Fragment::Initial(config_params) => Some(config_params),
                    _ => None,
                })
                .next()
                .expect("the Initial fragment to be present in the genesis block"),
        );

        let stable_storage = StableIndexShared::default();

        let block = ExplorerBlock::resolve_from(
            &block0,
            indexing::ExplorerBlockBuildingContext {
                discrimination: blockchain_config.discrimination,
                prev_transactions: &Transactions::new(),
                prev_blocks: &Blocks::new(),
                stable_storage: stable_storage.clone(),
            },
        )
        .await;

        let blocks = apply_block_to_blocks(Blocks::new(), &block)?;
        let epochs = apply_block_to_epochs(Epochs::new(), &block);
        let chain_lengths = apply_block_to_chain_lengths(ChainLengths::new(), &block)?;
        let transactions = apply_block_to_transactions(Transactions::new(), &block)?;
        let addresses = apply_block_to_addresses(Addresses::new(), &block);
        let (stake_pool_data, stake_pool_blocks) =
            apply_block_to_stake_pools(StakePool::new(), StakePoolBlocks::new(), &block);
        let vote_plans = apply_block_to_vote_plans(VotePlans::new(), &blockchain_tip, &block);

        let initial_state = State {
            transactions,
            blocks,
            epochs,
            chain_lengths,
            addresses,
            stake_pool_data,
            stake_pool_blocks,
            vote_plans,
        };

        let block0_id = block0.id();
        let (_, multiverse) = Multiverse::new(block0.chain_length(), block0_id, initial_state);

        let block0_id = block0.id();

        let maybe_head = blockchain.storage().get_tag(MAIN_BRANCH_TAG)?;
        let (stream, hash) = match maybe_head {
            Some(head) => (blockchain.storage().stream_from_to(block0_id, head)?, head),
            None => {
                return Err(Error::BootstrapError(
                    "Couldn't read the HEAD tag from storage".to_owned(),
                ))
            }
        };

        let (tx, _) = broadcast::channel(10);

        let bootstraped_db = ExplorerDb {
            multiverse,
            longest_chain_tip: Tip::new(hash),
            blockchain_config,
            blockchain: blockchain.clone(),
            blockchain_tip,
            stable_storage,
            tip_broadcast: tx,
        };

        let db = stream
            .map_err(Error::from)
            .try_fold(bootstraped_db, |db, block| async move {
                let block_id = block.id();
                db.apply_block(block).await?;
                // TODO: this only works because the StableIndex is in memory
                // otherwise, this would try to apply blocks there...
                // there are multiple solutions, but can change later
                db.set_tip(block_id).await?;
                Ok(db)
            })
            .await?;

        for branch in blockchain.branches().branches().await.iter() {
            let mut hash = branch.hash();
            let mut blocks = vec![];
            loop {
                if db.get_block(&hash).await.is_some() {
                    break;
                }
                let block = blockchain.storage().get(hash)?.ok_or_else(|| {
                    Error::BootstrapError(format!("couldn't get block {} from the storage", hash))
                })?;
                hash = block.header.block_parent_hash();
                blocks.push(block);
            }
            while let Some(block) = blocks.pop() {
                db.apply_block(block).await?;
            }
        }

        Ok(db)
    }

    /// Try to add a new block to the indexes, this can fail if the parent of the block is
    /// not processed. Also, update the longest seen chain with this block as tip if its
    /// chain length is greater than the current.
    /// This doesn't perform any validation on the given block and the previous state, it
    /// is assumed that the Block is valid
    async fn apply_block(&self, block: Block) -> Result<multiverse::Ref> {
        debug!(
            id=%block.id(),
            chain_length=%block.chain_length(),
            parent=%block.header.block_parent_hash(),
            "applying block to explorer's in-memory storage",
        );

        let previous_block = block.header.block_parent_hash();
        let chain_length = block.header.chain_length();
        let block_id = block.header.hash();
        let multiverse = self.multiverse.clone();
        let discrimination = self.blockchain_config.discrimination;

        let previous_state = multiverse
            .get_ref(&previous_block)
            .await
            .ok_or_else(|| Error::AncestorNotFound(block.id()))?;
        let State {
            transactions,
            blocks,
            addresses,
            epochs,
            chain_lengths,
            stake_pool_data,
            stake_pool_blocks,
            vote_plans,
        } = previous_state.state().clone();

        let explorer_block = ExplorerBlock::resolve_from(
            &block,
            indexing::ExplorerBlockBuildingContext {
                discrimination,
                prev_transactions: &transactions,
                prev_blocks: &blocks,
                stable_storage: self.stable_storage.clone(),
            },
        )
        .await;

        let (stake_pool_data, stake_pool_blocks) =
            apply_block_to_stake_pools(stake_pool_data, stake_pool_blocks, &explorer_block);

        let mut blocks = apply_block_to_blocks(blocks, &explorer_block)?;
        let mut addresses = apply_block_to_addresses(addresses, &explorer_block);
        let mut transactions = apply_block_to_transactions(transactions, &explorer_block)?;
        let mut chain_lengths = apply_block_to_chain_lengths(chain_lengths, &explorer_block)?;
        let mut epochs = apply_block_to_epochs(epochs, &explorer_block);

        let process_state =
            |blocks_to_invert: Option<VecDeque<Arc<ExplorerBlock>>>| -> Result<State> {
                for block_to_invert in blocks_to_invert.iter().flatten() {
                    blocks = unapply_block_to_blocks(blocks, block_to_invert.as_ref())?;
                    addresses = unapply_block_to_addresses(addresses, block_to_invert.as_ref());
                    transactions =
                        unapply_block_to_transactions(transactions, block_to_invert.as_ref())?;
                    chain_lengths =
                        unapply_block_to_chain_lengths(chain_lengths, block_to_invert.as_ref())?;
                    epochs = unapply_block_to_epochs(epochs, block_to_invert.as_ref());
                }

                Ok(State {
                    transactions,
                    blocks,
                    addresses,
                    epochs,
                    chain_lengths,
                    stake_pool_data,
                    stake_pool_blocks,
                    vote_plans: apply_block_to_vote_plans(
                        vote_plans,
                        &self.blockchain_tip,
                        &explorer_block,
                    ),
                })
            };

        let state_ref = multiverse
            .insert(chain_length, block.parent_id(), block_id, process_state)
            .await;

        state_ref
    }

    pub async fn get_block(&self, block_id: &HeaderHash) -> Option<Arc<ExplorerBlock>> {
        for (_, _hash, state_ref) in self.multiverse.tips().await.iter() {
            if let Some(b) = state_ref.state().blocks.lookup(&block_id) {
                return Some(Arc::clone(b));
            }
        }

        self.stable_storage
            .read()
            .await
            .get_block(block_id)
            .map(|block_ref| Arc::new(block_ref.clone()))
    }

    pub(self) async fn set_tip(&self, hash: HeaderHash) -> Result<bool> {
        // the tip changes which means now a block is confirmed (at least after
        // the initial epoch_stability_depth blocks).
        let state_ref = if let Some(state_ref) = self.multiverse.get_ref(&hash).await {
            state_ref
        } else {
            return Ok(false);
        };

        let block = {
            let state = state_ref.state();
            Arc::clone(state.blocks.lookup(&hash).unwrap())
        };

        if let Some(confirmed_block_chain_length) = block
            .chain_length()
            .nth_ancestor(self.blockchain_config.epoch_stability_depth)
        {
            let hash = state_ref
                .state()
                .chain_lengths
                .lookup(&confirmed_block_chain_length)
                .unwrap();

            let stable_block = Arc::clone(state_ref.state().blocks.lookup(&hash).unwrap());

            self.stable_storage
                .write()
                .await
                .apply_block((*stable_block).clone())?;

            self.multiverse.confirm_block(stable_block).await;

            // TODO: actually, maybe running gc with every tip change is not ideal?
            // maybe it's better to run it every X time or after N blocks
            self.multiverse
                .gc(self.blockchain_config.epoch_stability_depth)
                .await;
        }

        let mut guard = self.longest_chain_tip.0.write().await;

        debug!("setting explorer tip to: {}", hash);

        *guard = hash;

        let _ = self.tip_broadcast.send((hash, state_ref));

        Ok(true)
    }

    pub(self) async fn get_block_with_branches(
        &self,
        block_id: &HeaderHash,
    ) -> Option<(Arc<ExplorerBlock>, Vec<(HeaderHash, BranchQuery)>)> {
        let mut block = None;
        let mut tips = Vec::new();

        for (last_block, hash, state_ref) in self.multiverse.tips().await.drain(..) {
            if let Some(b) = state_ref.state().blocks.lookup(&block_id) {
                block = block.or_else(|| Some(Arc::clone(b)));
                tips.push((
                    hash,
                    BranchQuery {
                        state_ref,
                        stable_storage: self.stable_storage.clone(),
                        last_block,
                    },
                ));
            }
        }

        if block.is_some() {
            block.map(|b| (b, tips))
        } else {
            if let Some(block) = self.stable_storage.read().await.get_block(block_id) {
                // a confirmed block is technically in all branches
                // TODO: maybe it's better to have an enum for the result here
                Some((
                    Arc::new(block.clone()),
                    self.multiverse
                        .tips()
                        .await
                        .drain(..)
                        .map(|(last_block, hash, state_ref)| {
                            (
                                hash,
                                BranchQuery {
                                    state_ref,
                                    stable_storage: self.stable_storage.clone(),
                                    last_block,
                                },
                            )
                        })
                        .collect(),
                ))
            } else {
                None
            }
        }
    }

    pub async fn get_epoch(&self, epoch: Epoch) -> Option<EpochData> {
        let tips = self.multiverse.tips().await;
        let (_, _, state_ref) = &tips[0];

        let from_multiverse = state_ref
            .state()
            .epochs
            .lookup(&epoch)
            .map(|e| e.as_ref().clone());

        if from_multiverse.is_some() {
            from_multiverse
        } else {
            self.stable_storage
                .read()
                .await
                .get_epoch_data(&epoch)
                .map(|data| data.clone())
        }
    }

    pub async fn is_block_confirmed(&self, block_id: &HeaderHash) -> bool {
        self.stable_storage.read().await.get_block(block_id).is_some()
    }

    pub async fn find_blocks_by_chain_length(&self, chain_length: ChainLength) -> Vec<HeaderHash> {
        let mut hashes = Vec::new();

        for (_, _hash, state_ref) in self.multiverse.tips().await.iter() {
            if let Some(hash) = state_ref.state().chain_lengths.lookup(&chain_length) {
                hashes.push(**hash);
            }
        }

        if hashes.is_empty() {
            self.stable_storage
                .read()
                .await
                .get_block_by_chain_length(&chain_length)
                .map(|hash| vec![*hash])
                .unwrap_or_default()
        } else {
            hashes.sort_unstable();
            hashes.dedup();

            hashes
        }
    }

    pub async fn find_blocks_by_transaction(&self, transaction_id: &FragmentId) -> Vec<HeaderHash> {
        let mut txs: Vec<_> = self
            .multiverse
            .tips()
            .await
            .iter()
            .filter_map(|(_, _tip_hash, state_ref)| {
                state_ref
                    .state()
                    .transactions
                    .lookup(&transaction_id)
                    .map(|arc| *arc.clone())
            })
            .collect();

        if txs.is_empty() {
            self.stable_storage
                .read()
                .await
                .transaction_to_block(transaction_id)
                .map(|id| vec![*id])
                .unwrap_or_default()
        } else {
            txs.sort_unstable();
            txs.dedup();

            txs
        }
    }

    pub async fn get_stake_pool_blocks(
        &self,
        pool: &PoolId,
    ) -> Option<Arc<PersistentSequence<HeaderHash>>> {
        let pool = pool.clone();

        // this is a tricky query, one option would be to take a hash and return
        // only the blocks from a particular branch, but it's not like a stake
        // pool would produce inconsistent branches itself, although there may
        // be a need to know the blocks that a stake pool got in the main branch
        // too.
        // for the time being, this query uses the maximum, because the branch
        // that has more blocks from this particular stake pool has all the
        // blocks produced by it
        self.multiverse
            .tips()
            .await
            .iter()
            .filter_map(|(_, _hash, state_ref)| state_ref.state().stake_pool_blocks.lookup(&pool))
            .max_by_key(|seq| seq.len())
            .map(Arc::clone)
    }

    pub async fn get_stake_pool_data(&self, pool: &PoolId) -> Option<Arc<StakePoolData>> {
        let pool = pool.clone();

        for (_, _hash, state_ref) in self.multiverse.tips().await.iter() {
            if let Some(b) = state_ref.state().stake_pool_data.lookup(&pool) {
                return Some(Arc::clone(b));
            }
        }

        None
    }

    pub async fn get_vote_plan_by_id(
        &self,
        vote_plan_id: &VotePlanId,
    ) -> Option<Arc<ExplorerVotePlan>> {
        for (_, _hash, state_ref) in self.multiverse.tips().await.iter() {
            if let Some(b) = state_ref.state().vote_plans.lookup(&vote_plan_id) {
                return Some(Arc::clone(b));
            }
        }

        None
    }

    pub(self) async fn get_branch(&self, hash: &HeaderHash) -> Option<BranchQuery> {
        let state_ref = self.multiverse.get_ref(hash).await?;
        let last_block = state_ref.state().blocks.lookup(hash).unwrap().chain_length;

        Some(BranchQuery {
            state_ref,
            stable_storage: self.stable_storage.clone(),
            last_block,
        })
    }

    pub(self) async fn get_tip(&self) -> (HeaderHash, BranchQuery) {
        let hash = self.longest_chain_tip.get_block_id().await;
        let state_ref = self.multiverse.get_ref(&hash).await.unwrap();
        let last_block = state_ref.state().blocks.lookup(&hash).unwrap().chain_length;
        (
            hash,
            BranchQuery {
                state_ref: state_ref,
                stable_storage: self.stable_storage.clone(),
                last_block,
            },
        )
    }

    pub(self) async fn get_branches(&self) -> Vec<(HeaderHash, BranchQuery)> {
        self.multiverse
            .tips()
            .await
            .iter()
            .map(|(last_block, hash, state_ref)| {
                (
                    *hash,
                    BranchQuery {
                        state_ref: state_ref.clone(),
                        stable_storage: self.stable_storage.clone(),
                        last_block: *last_block,
                    },
                )
            })
            .collect()
    }

    fn blockchain(&self) -> &Blockchain {
        &self.blockchain
    }

    pub(self) fn tip_subscription(
        &self,
    ) -> impl Stream<
        Item = std::result::Result<
            (HeaderHash, BranchQuery),
            tokio_stream::wrappers::errors::BroadcastStreamRecvError,
        >,
    > {
        let stable_store = self.stable_storage.clone();
        tokio_stream::wrappers::BroadcastStream::new(self.tip_broadcast.subscribe()).map(
            move |item| {
                item.map(|(hash, state_ref)| {
                    let last_block = state_ref.state().blocks.lookup(&hash).unwrap().chain_length;
                    (
                        hash,
                        BranchQuery {
                            state_ref,
                            stable_storage: stable_store.clone(),
                            last_block,
                        },
                    )
                })
            },
        )
    }
}

fn apply_block_to_transactions(
    mut transactions: Transactions,
    block: &ExplorerBlock,
) -> Result<Transactions> {
    let block_id = block.id();
    let ids = block.transactions.values().map(|tx| tx.id());

    for id in ids {
        transactions = transactions
            .insert(id, Arc::new(block_id))
            .map_err(|_| Error::TransactionAlreadyExists(id))?;
    }

    Ok(transactions)
}

fn unapply_block_to_transactions(
    mut transactions: Transactions,
    block: &ExplorerBlock,
) -> Result<Transactions> {
    let ids = block.transactions.values().map(|tx| tx.id());

    for id in ids {
        transactions = transactions
            .remove(&id)
            .map_err(|_| Error::TransactionNotFound(id))?;
    }

    Ok(transactions)
}

fn apply_block_to_blocks(blocks: Blocks, block: &ExplorerBlock) -> Result<Blocks> {
    let block_id = block.id();
    blocks
        .insert(block_id, Arc::new(block.clone()))
        .map_err(|_| Error::BlockAlreadyExists(block_id))
}

fn unapply_block_to_blocks(blocks: Blocks, block: &ExplorerBlock) -> Result<Blocks> {
    let block_id = block.id();
    blocks
        .remove(&block_id)
        .map_err(|_| Error::BlockNotFound(block_id))
}

fn apply_block_to_addresses(mut addresses: Addresses, block: &ExplorerBlock) -> Addresses {
    let transactions = block.transactions.values();

    for tx in transactions {
        let id = tx.id();

        // A Hashset is used for preventing duplicates when the address is both an
        // input and an output in the given transaction

        let included_addresses: std::collections::HashSet<ExplorerAddress> = tx
            .outputs()
            .iter()
            .map(|output| output.address.clone())
            .chain(tx.inputs().iter().map(|input| input.address.clone()))
            .collect();

        for address in included_addresses {
            addresses = addresses.insert_or_update_simple(
                address,
                Arc::new(PersistentSequence::new().append(id)),
                |set| {
                    let new_set = set.append(id);
                    Some(Arc::new(new_set))
                },
            )
        }
    }
    addresses
}

fn unapply_block_to_addresses(mut addresses: Addresses, block: &ExplorerBlock) -> Addresses {
    let transactions = block.transactions.values();

    for tx in transactions {
        let id = tx.id();

        // A Hashset is used for preventing duplicates when the address is both an
        // input and an output in the given transaction

        let included_addresses: std::collections::HashSet<ExplorerAddress> = tx
            .outputs()
            .iter()
            .map(|output| output.address.clone())
            .chain(tx.inputs().iter().map(|input| input.address.clone()))
            .collect();

        for address in included_addresses {
            addresses = addresses
                .update::<_, Infallible>(&address, |set| {
                    Ok(set.remove_first().map(|(seq, removed)| {
                        assert_eq!(*removed, id);
                        Arc::new(seq)
                    }))
                })
                .unwrap()
        }
    }
    addresses
}

fn apply_block_to_epochs(epochs: Epochs, block: &ExplorerBlock) -> Epochs {
    let epoch_id = block.date().epoch;
    let block_id = block.id();

    epochs.insert_or_update_simple(
        epoch_id,
        Arc::new(EpochData {
            first_block: block_id,
            last_block: block_id,
            total_blocks: 0,
        }),
        |data| {
            Some(Arc::new(EpochData {
                first_block: data.first_block,
                last_block: block_id,
                total_blocks: data.total_blocks + 1,
            }))
        },
    )
}

fn unapply_block_to_epochs(epochs: Epochs, block: &ExplorerBlock) -> Epochs {
    let epoch_id = block.date().epoch;
    let block_id = block.id();

    let epoch_data = epochs.lookup(&epoch_id).unwrap();

    if epoch_data.last_block == block_id {
        epochs.remove(&epoch_id).unwrap()
    } else {
        epochs
    }
}

fn apply_block_to_chain_lengths(
    chain_lengths: ChainLengths,
    block: &ExplorerBlock,
) -> Result<ChainLengths> {
    let new_block_chain_length = block.chain_length();
    let new_block_hash = block.id();
    chain_lengths
        .insert(new_block_chain_length, Arc::new(new_block_hash))
        .map_err(|_| {
            // I think this shouldn't happen
            Error::ChainLengthBlockAlreadyExists(new_block_chain_length)
        })
}

fn unapply_block_to_chain_lengths(
    chain_lengths: ChainLengths,
    block: &ExplorerBlock,
) -> Result<ChainLengths> {
    let new_block_chain_length = block.chain_length();
    chain_lengths
        .remove(&new_block_chain_length)
        .map_err(|_| Error::BlockNotFound(block.id()))
}

fn apply_block_to_stake_pools(
    data: StakePool,
    blocks: StakePoolBlocks,
    block: &ExplorerBlock,
) -> (StakePool, StakePoolBlocks) {
    let mut blocks = match &block.producer() {
        indexing::BlockProducer::StakePool(id) => blocks
            .update(
                &id,
                |array: &Arc<PersistentSequence<HeaderHash>>| -> std::result::Result<_, Infallible> {
                    Ok(Some(Arc::new(array.append(block.id()))))
                },
            )
            .expect("block to be created by registered stake pool"),
        indexing::BlockProducer::BftLeader(_) => blocks,
        indexing::BlockProducer::None => blocks,
    };

    let mut data = data;

    for tx in block.transactions.values() {
        if let Some(cert) = &tx.certificate {
            blocks = match cert {
                Certificate::PoolRegistration(registration) => blocks
                    .insert(registration.to_id(), Arc::new(PersistentSequence::new()))
                    .expect("pool was registered more than once"),
                _ => blocks,
            };
            data = match cert {
                Certificate::PoolRegistration(registration) => data
                    .insert(
                        registration.to_id(),
                        Arc::new(StakePoolData {
                            registration: registration.clone(),
                            retirement: None,
                        }),
                    )
                    .expect("pool was registered more than once"),
                Certificate::PoolRetirement(retirement) => {
                    data.update::<_, Infallible>(&retirement.pool_id, |pool_data| {
                        Ok(Some(Arc::new(StakePoolData {
                            registration: pool_data.registration.clone(),
                            retirement: Some(retirement.clone()),
                        })))
                    })
                    .expect("pool was retired before registered");
                    data
                }
                _ => data,
            };
        }
    }

    (data, blocks)
}

fn apply_block_to_vote_plans(
    mut vote_plans: VotePlans,
    blockchain_tip: &blockchain::Tip,
    block: &ExplorerBlock,
) -> VotePlans {
    for tx in block.transactions.values() {
        if let Some(cert) = &tx.certificate {
            vote_plans = match cert {
                Certificate::VotePlan(vote_plan) => vote_plans
                    .insert(
                        vote_plan.to_id(),
                        Arc::new(ExplorerVotePlan {
                            id: vote_plan.to_id(),
                            vote_start: vote_plan.vote_start(),
                            vote_end: vote_plan.vote_end(),
                            committee_end: vote_plan.committee_end(),
                            payload_type: vote_plan.payload_type(),
                            proposals: vote_plan
                                .proposals()
                                .iter()
                                .map(|proposal| ExplorerVoteProposal {
                                    proposal_id: proposal.external_id().clone(),
                                    options: proposal.options().clone(),
                                    tally: None,
                                    votes: Default::default(),
                                })
                                .collect(),
                        }),
                    )
                    .unwrap(),
                Certificate::VoteCast(vote_cast) => {
                    use chain_impl_mockchain::vote::Payload;
                    let voter = tx.inputs[0].address.clone();
                    match vote_cast.payload() {
                        Payload::Public { choice } => vote_plans
                            .update(vote_cast.vote_plan(), |vote_plan| {
                                let mut proposals = vote_plan.proposals.clone();
                                proposals[vote_cast.proposal_index() as usize].votes = proposals
                                    [vote_cast.proposal_index() as usize]
                                    .votes
                                    .insert_or_update(
                                        voter,
                                        Arc::new(ExplorerVote::Public(*choice)),
                                        |_| {
                                            Ok::<_, std::convert::Infallible>(Some(Arc::new(
                                                ExplorerVote::Public(*choice),
                                            )))
                                        },
                                    )
                                    .unwrap();
                                let vote_plan = ExplorerVotePlan {
                                    proposals,
                                    ..(**vote_plan).clone()
                                };
                                Ok::<_, std::convert::Infallible>(Some(Arc::new(vote_plan)))
                            })
                            .unwrap(),
                        Payload::Private {
                            proof,
                            encrypted_vote,
                        } => vote_plans
                            .update(vote_cast.vote_plan(), |vote_plan| {
                                let mut proposals = vote_plan.proposals.clone();
                                proposals[vote_cast.proposal_index() as usize].votes = proposals
                                    [vote_cast.proposal_index() as usize]
                                    .votes
                                    .insert_or_update(
                                        voter,
                                        Arc::new(ExplorerVote::Private {
                                            proof: proof.clone(),
                                            encrypted_vote: encrypted_vote.clone(),
                                        }),
                                        |_| {
                                            Ok::<_, std::convert::Infallible>(Some(Arc::new(
                                                ExplorerVote::Private {
                                                    proof: proof.clone(),
                                                    encrypted_vote: encrypted_vote.clone(),
                                                },
                                            )))
                                        },
                                    )
                                    .unwrap();
                                let vote_plan = ExplorerVotePlan {
                                    proposals,
                                    ..(**vote_plan).clone()
                                };
                                Ok::<_, std::convert::Infallible>(Some(Arc::new(vote_plan)))
                            })
                            .unwrap(),
                    }
                }
                Certificate::VoteTally(vote_tally) => {
                    use chain_impl_mockchain::vote::PayloadType;
                    vote_plans
                        .update(vote_tally.id(), |vote_plan| {
                            let proposals_from_state =
                                futures::executor::block_on(blockchain_tip.get_ref())
                                    .active_vote_plans()
                                    .into_iter()
                                    .find_map(|vps| {
                                        if vps.id != vote_plan.id {
                                            return None;
                                        }
                                        Some(vps.proposals)
                                    })
                                    .unwrap();
                            let proposals = vote_plan
                                .proposals
                                .clone()
                                .into_iter()
                                .enumerate()
                                .map(|(index, mut proposal)| {
                                    proposal.tally = Some(match vote_tally.tally_type() {
                                        PayloadType::Public => ExplorerVoteTally::Public {
                                            results: proposals_from_state[index]
                                                .tally
                                                .clone()
                                                .unwrap()
                                                .result()
                                                .unwrap()
                                                .results()
                                                .to_vec(),
                                            options: proposal.options.clone(),
                                        },
                                        PayloadType::Private => ExplorerVoteTally::Private {
                                            results: proposals_from_state[index]
                                                .tally
                                                .clone()
                                                .unwrap()
                                                .result()
                                                .map(|tally_results| {
                                                    tally_results.results().to_vec()
                                                }),
                                            options: proposal.options.clone(),
                                        },
                                    });
                                    proposal
                                })
                                .collect();
                            let vote_plan = ExplorerVotePlan {
                                proposals,
                                ..(**vote_plan).clone()
                            };
                            Ok::<_, std::convert::Infallible>(Some(Arc::new(vote_plan)))
                        })
                        .unwrap()
                }
                _ => vote_plans,
            }
        }
    }

    vote_plans
}

impl BlockchainConfig {
    fn from_config_params(params: &ConfigParams) -> BlockchainConfig {
        let mut discrimination: Option<Discrimination> = None;
        let mut consensus_version: Option<ConsensusVersion> = None;
        let mut fees: Option<LinearFee> = None;
        let mut epoch_stability_depth: Option<u32> = None;

        for p in params.iter() {
            match p {
                ConfigParam::Discrimination(d) => {
                    discrimination.replace(*d);
                }
                ConfigParam::ConsensusVersion(v) => {
                    consensus_version.replace(*v);
                }
                ConfigParam::LinearFee(fee) => {
                    fees.replace(*fee);
                }
                ConfigParam::EpochStabilityDepth(d) => {
                    epoch_stability_depth.replace(*d);
                }
                _ => (),
            }
        }

        BlockchainConfig {
            discrimination: discrimination.expect("discrimination not found in initial params"),
            consensus_version: consensus_version
                .expect("consensus version not found in initial params"),
            fees: fees.expect("fees not found in initial params"),
            epoch_stability_depth: epoch_stability_depth
                .expect("epoch stability depth not found in initial params"),
        }
    }
}

impl Tip {
    fn new(block0_hash: HeaderHash) -> Tip {
        Tip(Arc::new(RwLock::new(block0_hash)))
    }

    async fn get_block_id(&self) -> HeaderHash {
        *self.0.read().await
    }
}

/// wrapper used to contextualize queries within a particular branch
/// this tries to search first in memory (the state_ref), in case that fails, it
/// tries with the stable index (but it is not a cache, because the two datasets
/// are disjoint)
#[derive(Clone)]
pub struct BranchQuery {
    state_ref: multiverse::Ref,
    stable_storage: StableIndexShared,
    // TODO: this could be embedded/cached in the state, it's a
    // performance/memory tradeoff, analyze later
    last_block: ChainLength,
}

impl BranchQuery {
    pub async fn get_block(&self, block_id: &HeaderHash) -> Option<Arc<ExplorerBlock>> {
        self.state_ref
            .state()
            .blocks
            .lookup(&block_id)
            .cloned()
            .or(self
                .stable_storage
                .read()
                .await
                .get_block(block_id)
                .map(|block| Arc::new(block.clone())))
    }

    pub fn last_block(&self) -> ChainLength {
        self.last_block
    }

    pub fn get_vote_plans(&self) -> Vec<(VotePlanId, Arc<ExplorerVotePlan>)> {
        self.state_ref
            .state()
            .vote_plans
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    pub fn get_stake_pools(&self) -> Vec<(PoolId, Arc<StakePoolData>)> {
        self.state_ref
            .state()
            .stake_pool_data
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    pub fn transactions_by_address(
        &self,
        address: &ExplorerAddress,
    ) -> Option<PersistentSequence<FragmentId>> {
        self.state_ref
            .state()
            .addresses
            .lookup(address)
            .map(|txs| PersistentSequence::clone(txs))
    }

    /// Get the hashes of all blocks in the range [from, to)
    /// the ChainLength is returned to for easy of use in the case where
    /// `to` is greater than the max
    pub async fn get_block_hash_range(
        &self,
        from: ChainLength,
        to: ChainLength,
    ) -> Vec<(HeaderHash, ChainLength)> {
        let a = u32::from(from);
        let b = u32::from(to);

        let mut unstable: Vec<_> = (a..b)
            .filter_map(|i| {
                self.state_ref
                    .state()
                    .chain_lengths
                    .lookup(&i.into())
                    .map(|b| (*b.as_ref(), i.into()))
            })
            .collect();

        let stable_upper_bound = unstable.get(0).map(|(_, l)| *l).unwrap_or(to);
        let missing_in_unstable = stable_upper_bound != from;

        if missing_in_unstable {
            let stable_store = self.stable_storage.read().await;

            let blocks = stable_store.get_block_hash_range(from, stable_upper_bound);

            blocks.chain(unstable.drain(..)).collect()
        } else {
            unstable
        }
    }
}
