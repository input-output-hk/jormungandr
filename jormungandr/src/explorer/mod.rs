pub mod error;
pub mod graphql;
mod indexing;
mod persistent_sequence;

use self::error::{Error, ErrorKind, Result};
use self::graphql::Context;
use self::indexing::{
    Addresses, Blocks, ChainLengths, EpochData, Epochs, ExplorerAddress, ExplorerBlock, StakePool,
    StakePoolBlocks, StakePoolData, Transactions,
};
use self::persistent_sequence::PersistentSequence;

use crate::blockcfg::{
    Block, ChainLength, ConfigParam, ConfigParams, ConsensusVersion, Epoch, Fragment, FragmentId,
    HeaderHash,
};
use crate::blockchain::{Blockchain, Multiverse, MAIN_BRANCH_TAG};
use crate::intercom::ExplorerMsg;
use crate::utils::task::{Input, TokioServiceInfo};
use chain_addr::Discrimination;
use chain_core::property::Block as _;
use chain_impl_mockchain::certificate::{Certificate, PoolId};
use chain_impl_mockchain::fee::LinearFee;
use chain_impl_mockchain::multiverse;
use futures03::{compat::*, stream::TryStreamExt};
use std::convert::Infallible;
use std::sync::Arc;
use tokio::prelude::*;
use tokio::sync::lock::{Lock, LockGuard};

#[derive(Clone)]
pub struct Explorer {
    pub db: ExplorerDB,
    pub schema: Arc<graphql::Schema>,
}

struct Branch {
    state_ref: multiverse::Ref<State>,
    length: ChainLength,
}

#[derive(Clone)]
struct Tip(Lock<Branch>);

#[derive(Clone)]
pub struct ExplorerDB {
    /// Structure that keeps all the known states to allow easy branch management
    /// each new block is indexed by getting its previous `State` from the multiverse
    /// and inserted a new updated one.
    multiverse: Multiverse<State>,
    /// This keeps track of the longest chain seen until now. All the queries are
    /// performed using the state of this branch, the HeaderHash is used as key for the
    /// multiverse, and the ChainLength is used in the updating process.
    longest_chain_tip: Tip,
    pub blockchain_config: BlockchainConfig,
    blockchain: Blockchain,
}

#[derive(Clone)]
pub struct BlockchainConfig {
    /// Used to construct `Address` from `AccountIndentifier` when processing transaction
    /// inputs
    discrimination: Discrimination,
    consensus_version: ConsensusVersion,
    fees: LinearFee,
}

/// Inmutable data structure used to represent the explorer's state at a given Block
/// A new state can be obtained to from a Block and it's previous state, getting two
/// independent states but with memory sharing to minimize resource utilization
#[derive(Clone)]
struct State {
    parent_ref: Option<multiverse::Ref<State>>,
    transactions: Transactions,
    blocks: Blocks,
    addresses: Addresses,
    epochs: Epochs,
    chain_lengths: ChainLengths,
    stake_pool_data: StakePool,
    stake_pool_blocks: StakePoolBlocks,
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
    pub fn new(db: ExplorerDB, schema: graphql::Schema) -> Explorer {
        Explorer {
            db,
            schema: Arc::new(schema),
        }
    }

    pub fn context(&self) -> Context {
        Context {
            db: self.db.clone(),
            settings: Settings {
                // Hardcoded bech32 prefix
                address_bech32_prefix: "addr".to_owned(),
            },
        }
    }

    pub fn handle_input(
        &mut self,
        info: &TokioServiceInfo,
        input: Input<ExplorerMsg>,
    ) -> impl Future<Item = (), Error = ()> {
        let _logger = info.logger();
        let bquery = match input {
            Input::Shutdown => {
                return future::ok(());
            }
            Input::Input(msg) => msg,
        };

        let mut explorer_db = self.db.clone();
        let logger = info.logger().clone();
        match bquery {
            ExplorerMsg::NewBlock(block) => info.spawn(
                "apply block",
                explorer_db
                    .apply_block(block)
                    .then(move |result| match result {
                        // XXX: There is no garbage collection now, so the GCRoot is not used
                        Ok(_gc_root) => Ok(()),
                        Err(err) => Err(error!(logger, "Explorer error: {}", err)),
                    }),
            ),
        }
        future::ok::<(), ()>(())
    }
}

impl ExplorerDB {
    /// Apply all the blocks in the [block0, MAIN_BRANCH_TAG], also extract the static
    /// Blockchain settings from the Block0 (Discrimination)
    /// This function is only called once on the node's bootstrap phase
    pub async fn bootstrap(block0: Block, blockchain: &Blockchain) -> Result<Self> {
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
            blockchain_config.discrimination,
            &Transactions::new(),
            &Blocks::new(),
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
        let initial_state_ref = multiverse
            .insert(block0.chain_length(), block0_id, initial_state)
            .compat()
            .await
            .expect("The multiverse to be empty");

        let bootstraped_db = ExplorerDB {
            multiverse,
            longest_chain_tip: Tip::new(Branch {
                state_ref: initial_state_ref,
                length: block0.header.chain_length(),
            }),
            blockchain_config,
            blockchain: blockchain.clone(),
        };

        let maybe_head = blockchain
            .storage()
            .get_tag(MAIN_BRANCH_TAG.to_owned())
            .await?;
        let stream = match maybe_head {
            Some(head) => blockchain.storage().stream_from_to(block0_id, head).await?,
            None => {
                return Err(Error::from(ErrorKind::BootstrapError(
                    "Couldn't read the HEAD tag from storage".to_owned(),
                )))
            }
        };
        stream
            .map_err(|err| Error::from(err))
            .try_fold(bootstraped_db, |mut db, block| {
                db.apply_block(block).and_then(|_gc_root| Ok(db)).compat()
            })
            .await
    }

    /// Try to add a new block to the indexes, this can fail if the parent of the block is
    /// not processed. Also, update the longest seen chain with this block as tip if its
    /// chain length is greater than the current.
    /// This doesn't perform any validation on the given block and the previous state, it
    /// is assumed that the Block is valid
    fn apply_block(
        &mut self,
        block: Block,
    ) -> impl Future<Item = multiverse::Ref<State>, Error = Error> {
        let previous_block = block.header.block_parent_hash();
        let chain_length = block.header.chain_length();
        let block_id = block.header.hash();
        let multiverse = self.multiverse.clone();
        let current_tip = self.longest_chain_tip.clone();
        let discrimination = self.blockchain_config.discrimination.clone();

        multiverse
            .get_ref(previous_block)
            .map_err(|_: Infallible| unreachable!())
            .and_then(move |maybe_previous_state| match maybe_previous_state {
                Some(state_ref) => {
                    let State {
                        parent_ref: _,
                        transactions,
                        blocks,
                        addresses,
                        epochs,
                        chain_lengths,
                        stake_pool_data,
                        stake_pool_blocks,
                    } = state_ref.state().clone();

                    let explorer_block =
                        ExplorerBlock::resolve_from(&block, discrimination, &transactions, &blocks);

                    Ok((
                        state_ref,
                        apply_block_to_transactions(transactions, &explorer_block)?,
                        apply_block_to_blocks(blocks, &explorer_block)?,
                        apply_block_to_addresses(addresses, &explorer_block)?,
                        apply_block_to_epochs(epochs, &explorer_block),
                        apply_block_to_chain_lengths(chain_lengths, &explorer_block)?,
                        apply_block_to_stake_pools(
                            stake_pool_data,
                            stake_pool_blocks,
                            &explorer_block,
                        ),
                    ))
                }
                None => Err(Error::from(ErrorKind::AncestorNotFound(format!(
                    "{}",
                    block.id()
                )))),
            })
            .and_then(
                move |(
                    parent_ref,
                    transactions,
                    blocks,
                    addresses,
                    epochs,
                    chain_lengths,
                    stake_pools,
                )| {
                    let chain_length = chain_length.clone();
                    let block_id = block_id.clone();
                    let (stake_pool_data, stake_pool_blocks) = stake_pools;
                    multiverse
                        .insert(
                            chain_length,
                            block_id,
                            State {
                                parent_ref: Some(parent_ref),
                                transactions,
                                blocks,
                                addresses,
                                epochs,
                                chain_lengths,
                                stake_pool_data,
                                stake_pool_blocks,
                            },
                        )
                        .map_err(|_: Infallible| unreachable!())
                        .map(move |state_ref| (state_ref, chain_length))
                },
            )
            .and_then(move |(state_ref, chain_length)| {
                current_tip
                    .compare_and_replace(Branch {
                        state_ref: state_ref.clone(),
                        length: chain_length,
                    })
                    .map_err(|_: Infallible| unreachable!())
                    .map(|_| state_ref)
            })
    }

    pub fn get_latest_block_hash(&self) -> impl Future<Item = HeaderHash, Error = Infallible> {
        self.longest_chain_tip.get_block_id()
    }

    pub fn get_block(
        &self,
        block_id: &HeaderHash,
    ) -> impl Future<Item = Option<ExplorerBlock>, Error = Infallible> {
        let block_id = block_id.clone();
        self.with_latest_state(move |state| {
            state.blocks.lookup(&block_id).map(|b| b.as_ref().clone())
        })
    }

    pub fn get_epoch(
        &self,
        epoch: Epoch,
    ) -> impl Future<Item = Option<EpochData>, Error = Infallible> {
        let epoch = epoch.clone();
        self.with_latest_state(move |state| state.epochs.lookup(&epoch).map(|e| e.as_ref().clone()))
    }

    pub fn find_block_by_chain_length(
        &self,
        chain_length: ChainLength,
    ) -> impl Future<Item = Option<HeaderHash>, Error = Infallible> {
        self.with_latest_state(move |state| {
            state
                .chain_lengths
                .lookup(&chain_length)
                .map(|b| b.as_ref().clone())
        })
    }

    pub fn find_block_hash_by_transaction(
        &self,
        transaction_id: &FragmentId,
    ) -> impl Future<Item = Option<HeaderHash>, Error = Infallible> {
        let transaction_id = transaction_id.clone();
        self.with_latest_state(move |state| {
            state
                .transactions
                .lookup(&transaction_id)
                .map(|id| id.as_ref().clone())
        })
    }

    pub fn get_transactions_by_address(
        &self,
        address: &ExplorerAddress,
    ) -> impl Future<Item = Option<PersistentSequence<FragmentId>>, Error = Infallible> {
        let address = address.clone();
        self.with_latest_state(move |state| {
            state
                .addresses
                .lookup(&address)
                .map(|set| set.as_ref().clone())
        })
    }

    // Get the hashes of all blocks in the range [from, to)
    // the ChainLength is returned to for easy of use in the case where
    // `to` is greater than the max
    pub fn get_block_hash_range(
        &self,
        from: ChainLength,
        to: ChainLength,
    ) -> impl Future<Item = Vec<(HeaderHash, ChainLength)>, Error = Infallible> {
        let from = u32::from(from);
        let to = u32::from(to);

        self.with_latest_state(move |state| {
            (from..to)
                .filter_map(|i| {
                    state
                        .chain_lengths
                        .lookup(&i.into())
                        .map(|b| (b.as_ref().clone(), i.into()))
                })
                .collect()
        })
    }

    pub fn get_stake_pool_blocks(
        &self,
        pool: &PoolId,
    ) -> impl Future<Item = Option<PersistentSequence<HeaderHash>>, Error = Infallible> {
        let pool = pool.clone();
        self.with_latest_state(move |state| {
            state
                .stake_pool_blocks
                .lookup(&pool)
                .map(|i| i.as_ref().clone())
        })
    }

    pub fn get_stake_pool_data(
        &self,
        pool: &PoolId,
    ) -> impl Future<Item = Option<StakePoolData>, Error = Infallible> {
        let pool = pool.clone();
        self.with_latest_state(move |state| {
            state
                .stake_pool_data
                .lookup(&pool)
                .map(|i| i.as_ref().clone())
        })
    }

    pub fn get_stake_pools(
        &self,
    ) -> impl Future<Item = Vec<(PoolId, Arc<StakePoolData>)>, Error = Infallible> {
        self.with_latest_state(move |state| {
            state
                .stake_pool_data
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect()
        })
    }

    /// run given function with the longest branch's state
    fn with_latest_state<T>(
        &self,
        f: impl Fn(State) -> T,
    ) -> impl Future<Item = T, Error = Infallible> {
        let multiverse = self.multiverse.clone();
        self.get_latest_block_hash().and_then(move |branch_id| {
            multiverse.get(branch_id).and_then(move |maybe_state| {
                let state = maybe_state.expect("the longest chain to be indexed");
                Ok(f(state))
            })
        })
    }

    fn blockchain(&self) -> &Blockchain {
        &self.blockchain
    }
}

fn get_lock<L>(lock: &Lock<L>) -> impl Future<Item = LockGuard<L>, Error = Infallible> {
    let mut lock = (*lock).clone();
    future::poll_fn(move || Ok(lock.poll_lock()))
}

fn apply_block_to_transactions(
    mut transactions: Transactions,
    block: &ExplorerBlock,
) -> Result<Transactions> {
    let block_id = block.id();
    let ids = block.transactions.values().map(|tx| tx.id());

    for id in ids {
        transactions = transactions
            .insert(id, Arc::new(block_id.clone()))
            .map_err(|_| ErrorKind::TransactionAlreadyExists(format!("{}", id)))?;
    }

    Ok(transactions)
}

fn apply_block_to_blocks(blocks: Blocks, block: &ExplorerBlock) -> Result<Blocks> {
    let block_id = block.id();
    blocks
        .insert(block_id, Arc::new(block.clone()))
        .map_err(|_| Error::from(ErrorKind::BlockAlreadyExists(format!("{}", block_id))))
}

fn apply_block_to_addresses(mut addresses: Addresses, block: &ExplorerBlock) -> Result<Addresses> {
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
                Arc::new(PersistentSequence::new().append(id.clone())),
                |set| {
                    let new_set = set.append(id.clone());
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
) -> Result<ChainLengths> {
    let new_block_chain_length = block.chain_length();
    let new_block_hash = block.id();
    chain_lengths
        .insert(new_block_chain_length, Arc::new(new_block_hash))
        .map_err(|_| {
            // I think this shouldn't happen
            Error::from(ErrorKind::ChainLengthBlockAlreadyExists(u32::from(
                new_block_chain_length,
            )))
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
        indexing::BlockProducer::BftLeader(_) => unimplemented!(),
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
                ConfigParam::Discrimination(discrimination) => Some(discrimination.clone()),
                _ => None,
            })
            .next()
            .expect("the discrimination to be present");

        let consensus_version = params
            .iter()
            .filter_map(|param| match param {
                ConfigParam::ConsensusVersion(version) => Some(version.clone()),
                _ => None,
            })
            .next()
            .expect("consensus version to be present");

        let fees = params
            .iter()
            .filter_map(|param| match param {
                ConfigParam::LinearFee(fee) => Some(fee.clone()),
                _ => None,
            })
            .next()
            .expect("fee is not in config params");

        BlockchainConfig {
            discrimination,
            consensus_version,
            fees,
        }
    }
}

impl Tip {
    fn new(branch: Branch) -> Tip {
        Tip(Lock::new(branch))
    }

    fn compare_and_replace(&self, other: Branch) -> impl Future<Item = (), Error = Infallible> {
        get_lock(&self.0).and_then(move |mut current| {
            // Probably a different thing is needed for the == case
            if other.length > (*current).length {
                *current = Branch {
                    state_ref: other.state_ref,
                    length: other.length,
                };
            }
            Ok(())
        })
    }

    fn get_block_id(&self) -> impl Future<Item = HeaderHash, Error = Infallible> {
        get_lock(&self.0).map(|guard| *guard.state_ref.id())
    }
}
