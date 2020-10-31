pub mod error;
pub mod indexing;
pub mod persistent_sequence;

use self::error::Error;
use self::indexing::{
    Addresses, Blocks, ChainLengths, EpochData, Epochs, ExplorerAddress, ExplorerBlock, StakePool,
    StakePoolBlocks, StakePoolData, Transactions,
};
use self::persistent_sequence::PersistentSequence;
use chain_addr::Discrimination;
use chain_core::property::Block as _;
use chain_impl_mockchain::{
    block::{Block, ChainLength},
    certificate::{Certificate, PoolId},
    chaintypes::ConsensusVersion,
    config::ConfigParam,
    fee::LinearFee,
    fragment::{ConfigParams, Fragment, FragmentId},
    header::{Epoch, HeaderId as HeaderHash},
    multiverse,
};
use std::convert::Infallible;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct DB {
    /// Structure that keeps all the known states to allow easy branch management
    /// each new block is indexed by getting its previous `State` from the multiverse
    /// and inserted a new updated one.
    multiverse: Multiverse<State>,
    pub blockchain_config: BlockchainConfig,
    pub tip: HeaderHash,
}

#[derive(Clone)]
pub struct BlockchainConfig {
    /// Used to construct `Address` from `AccountIndentifier` when processing transaction
    /// inputs
    pub discrimination: Discrimination,
    pub consensus_version: ConsensusVersion,
    pub fees: LinearFee,
}

/// Inmutable data structure used to represent the explorer's state at a given Block
/// A new state can be obtained to from a Block and it's previous state, getting two
/// independent states but with memory sharing to minimize resource utilization
#[derive(Clone)]
pub struct State {
    parent_ref: Option<multiverse::Ref<State>>,
    transactions: Transactions,
    blocks: Blocks,
    addresses: Addresses,
    epochs: Epochs,
    chain_lengths: ChainLengths,
    stake_pool_data: StakePool,
    stake_pool_blocks: StakePoolBlocks,
}

impl DB {
    /// Apply all the blocks in the [block0, MAIN_BRANCH_TAG], also extract the static
    /// Blockchain settings from the Block0 (Discrimination)
    /// This function is only called once on the node's bootstrap phase
    pub async fn bootstrap(block0: Block) -> Result<Self, Error> {
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

        let block = ExplorerBlock::resolve_from(
            &block0,
            indexing::ExplorerBlockBuildingContext {
                discrimination: blockchain_config.discrimination,
                prev_transactions: &Transactions::new(),
                prev_blocks: &Blocks::new(),
            },
        );

        let blocks = apply_block_to_blocks(Blocks::new(), &block)?;
        let epochs = apply_block_to_epochs(Epochs::new(), &block);
        let chain_lengths = apply_block_to_chain_lengths(ChainLengths::new(), &block)?;
        let transactions = apply_block_to_transactions(Transactions::new(), &block)?;
        let addresses = apply_block_to_addresses(Addresses::new(), &block)?;
        let (stake_pool_data, stake_pool_blocks) =
            apply_block_to_stake_pools(StakePool::new(), StakePoolBlocks::new(), &block);

        let initial_state = State {
            blocks,
            epochs,
            chain_lengths,
            transactions,
            addresses,
            stake_pool_data,
            stake_pool_blocks,
            parent_ref: None,
        };

        let multiverse = Multiverse::<State>::new();
        let block0_id = block0.id();
        let _initial_state_ref = multiverse
            .insert(block0.chain_length(), block0_id, initial_state)
            .await;

        let bootstraped_db = DB {
            multiverse,
            blockchain_config,
            tip: block0_id,
        };

        Ok(bootstraped_db)
    }

    /// Try to add a new block to the indexes, this can fail if the parent of the block is
    /// not processed. Also, update the longest seen chain with this block as tip if its
    /// chain length is greater than the current.
    /// This doesn't perform any validation on the given block and the previous state, it
    /// is assumed that the Block is valid
    pub async fn apply_block(&mut self, block: Block) -> Result<multiverse::Ref<State>, Error> {
        let previous_block = block.header.block_parent_hash();
        let chain_length = block.header.chain_length();
        let block_id = block.header.hash();
        let multiverse = self.multiverse.clone();
        let discrimination = self.blockchain_config.discrimination;

        let previous_state: multiverse::Ref<State> = multiverse
            .get_ref(previous_block)
            .await
            .ok_or_else(|| Error::AncestorNotFound(format!("{}", block.id())))?;

        let State {
            parent_ref: _,
            transactions,
            blocks,
            addresses,
            epochs,
            chain_lengths,
            stake_pool_data,
            stake_pool_blocks,
        } = previous_state.state().clone();

        let explorer_block = ExplorerBlock::resolve_from(
            &block,
            indexing::ExplorerBlockBuildingContext {
                discrimination,
                prev_transactions: &transactions,
                prev_blocks: &blocks,
            },
        );
        let (stake_pool_data, stake_pool_blocks) =
            apply_block_to_stake_pools(stake_pool_data, stake_pool_blocks, &explorer_block);

        let state_ref = multiverse
            .insert(
                chain_length,
                block_id,
                State {
                    parent_ref: Some(previous_state),
                    transactions: apply_block_to_transactions(transactions, &explorer_block)?,
                    blocks: apply_block_to_blocks(blocks, &explorer_block)?,
                    addresses: apply_block_to_addresses(addresses, &explorer_block)?,
                    epochs: apply_block_to_epochs(epochs, &explorer_block),
                    chain_lengths: apply_block_to_chain_lengths(chain_lengths, &explorer_block)?,
                    stake_pool_data,
                    stake_pool_blocks,
                },
            )
            .await;

        Ok(state_ref)
    }

    pub async fn get_latest_block_hash(&self) -> HeaderHash {
        self.tip
    }

    pub async fn set_tip(&mut self, tip: HeaderHash) {
        self.tip = tip;
    }

    pub async fn get_block(&self, block_id: &HeaderHash) -> Option<ExplorerBlock> {
        let block_id = *block_id;
        self.with_state(block_id, move |state| {
            state.and_then(|state| state.blocks.lookup(&block_id).map(|b| b.as_ref().clone()))
        })
        .await
    }

    pub async fn get_epoch(&self, epoch: Epoch) -> Option<EpochData> {
        self.with_latest_state(move |state| state.epochs.lookup(&epoch).map(|e| e.as_ref().clone()))
            .await
    }

    pub async fn find_block_by_chain_length(
        &self,
        chain_length: ChainLength,
    ) -> Option<HeaderHash> {
        self.with_latest_state(move |state| {
            state
                .chain_lengths
                .lookup(&chain_length)
                .map(|b| *b.as_ref())
        })
        .await
    }

    pub async fn find_block_hash_by_transaction(
        &self,
        transaction_id: &FragmentId,
    ) -> Option<HeaderHash> {
        self.with_latest_state(move |state| {
            state
                .transactions
                .lookup(&transaction_id)
                .map(|id| *id.as_ref())
        })
        .await
    }

    pub async fn get_transactions_by_address(
        &self,
        address: &ExplorerAddress,
    ) -> Option<PersistentSequence<FragmentId>> {
        let address = address.clone();
        self.with_latest_state(move |state| {
            state
                .addresses
                .lookup(&address)
                .map(|set| set.as_ref().clone())
        })
        .await
    }

    // Get the hashes of all blocks in the range [from, to)
    // the ChainLength is returned to for easy of use in the case where
    // `to` is greater than the max
    pub async fn get_block_hash_range(
        &self,
        from: ChainLength,
        to: ChainLength,
    ) -> Vec<(HeaderHash, ChainLength)> {
        let from = u32::from(from);
        let to = u32::from(to);

        self.with_latest_state(move |state| {
            (from..to)
                .filter_map(|i| {
                    state
                        .chain_lengths
                        .lookup(&i.into())
                        .map(|b| (*b.as_ref(), i.into()))
                })
                .collect()
        })
        .await
    }

    pub async fn get_stake_pool_blocks(
        &self,
        pool: &PoolId,
    ) -> Option<PersistentSequence<HeaderHash>> {
        let pool = pool.clone();
        self.with_latest_state(move |state| {
            state
                .stake_pool_blocks
                .lookup(&pool)
                .map(|i| i.as_ref().clone())
        })
        .await
    }

    pub async fn get_stake_pool_data(&self, pool: &PoolId) -> Option<StakePoolData> {
        let pool = pool.clone();
        self.with_latest_state(move |state| {
            state
                .stake_pool_data
                .lookup(&pool)
                .map(|i| i.as_ref().clone())
        })
        .await
    }

    pub async fn get_stake_pools(&self) -> Vec<(PoolId, Arc<StakePoolData>)> {
        self.with_latest_state(move |state| {
            state
                .stake_pool_data
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect()
        })
        .await
    }

    /// run given function with the longest branch's state
    async fn with_latest_state<T>(&self, f: impl Fn(State) -> T) -> T {
        let multiverse = self.multiverse.clone();
        let branch_id = self.get_latest_block_hash().await;
        let maybe_state = multiverse.get(branch_id).await;
        let state = maybe_state.expect("the longest chain to be indexed");
        f(state)
    }

    async fn with_state<T>(&self, branch: HeaderHash, f: impl Fn(Option<State>) -> T) -> T {
        let multiverse = self.multiverse.clone();
        let maybe_state = multiverse.get(branch).await;
        f(maybe_state)
    }
}

fn apply_block_to_transactions(
    mut transactions: Transactions,
    block: &ExplorerBlock,
) -> Result<Transactions, Error> {
    let block_id = block.id();
    let ids = block.transactions.values().map(|tx| tx.id());

    for id in ids {
        transactions = transactions
            .insert(id, Arc::new(block_id))
            .map_err(|_| Error::TransactionAlreadyExists(format!("{}", id)))?;
    }

    Ok(transactions)
}

fn apply_block_to_blocks(blocks: Blocks, block: &ExplorerBlock) -> Result<Blocks, Error> {
    let block_id = block.id();
    blocks
        .insert(block_id, Arc::new(block.clone()))
        .map_err(|_| Error::BlockAlreadyExists(format!("{}", block_id)))
}

fn apply_block_to_addresses(
    mut addresses: Addresses,
    block: &ExplorerBlock,
) -> Result<Addresses, Error> {
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
    Ok(addresses)
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

fn apply_block_to_chain_lengths(
    chain_lengths: ChainLengths,
    block: &ExplorerBlock,
) -> Result<ChainLengths, Error> {
    let new_block_chain_length = block.chain_length();
    let new_block_hash = block.id();
    chain_lengths
        .insert(new_block_chain_length, Arc::new(new_block_hash))
        .map_err(|_| {
            // I think this shouldn't happen
            Error::ChainLengthBlockAlreadyExists(u32::from(new_block_chain_length))
        })
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
                    data.update::<_, ()>(&retirement.pool_id, |pool_data| {
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

impl BlockchainConfig {
    fn from_config_params(params: &ConfigParams) -> BlockchainConfig {
        let discrimination = params
            .iter()
            .filter_map(|param| match param {
                ConfigParam::Discrimination(discrimination) => Some(discrimination),
                _ => None,
            })
            .next()
            .expect("the discrimination to be present");

        let consensus_version = params
            .iter()
            .filter_map(|param| match param {
                ConfigParam::ConsensusVersion(version) => Some(version),
                _ => None,
            })
            .next()
            .expect("consensus version to be present");

        let fees = params
            .iter()
            .filter_map(|param| match param {
                ConfigParam::LinearFee(fee) => Some(fee),
                _ => None,
            })
            .next()
            .expect("fee is not in config params");

        BlockchainConfig {
            discrimination: *discrimination,
            consensus_version: *consensus_version,
            fees: *fees,
        }
    }
}

pub struct Multiverse<T> {
    inner: Arc<RwLock<multiverse::Multiverse<T>>>,
}

impl<T> Multiverse<T> {
    pub fn new() -> Self {
        Multiverse {
            inner: Arc::new(RwLock::new(multiverse::Multiverse::new())),
        }
    }

    pub async fn insert(
        &self,
        chain_length: ChainLength,
        hash: HeaderHash,
        value: T,
    ) -> multiverse::Ref<T> {
        let mut guard = self.inner.write().await;
        guard.insert(chain_length, hash, value)
    }

    pub async fn get_ref(&self, hash: HeaderHash) -> Option<multiverse::Ref<T>> {
        let guard = self.inner.read().await;
        guard.get_ref(&hash)
    }
}

impl<T: Clone> Multiverse<T> {
    pub async fn get(&self, hash: HeaderHash) -> Option<T> {
        let guard = self.inner.read().await;
        guard.get(&hash).as_deref().cloned()
    }
}

impl<T> Clone for Multiverse<T> {
    fn clone(&self) -> Self {
        Multiverse {
            inner: self.inner.clone(),
        }
    }
}
