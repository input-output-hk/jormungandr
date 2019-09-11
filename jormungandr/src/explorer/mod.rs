pub mod error;
pub mod graphql;

use self::error::{Error, ErrorKind, Result};
use self::graphql::Context;
use crate::blockcfg::{
    Block, ChainLength, ConfigParam, ConfigParams, ConsensusVersion, Epoch, Fragment, FragmentId,
    HeaderHash,
};
use crate::blockchain::Multiverse;
use crate::intercom::ExplorerMsg;
use crate::utils::task::{Input, TokioServiceInfo};
use chain_addr::{Address, Discrimination};
use chain_core::property::Block as _;
use chain_core::property::Fragment as _;
use chain_impl_mockchain::multiverse::GCRoot;
use chain_impl_mockchain::transaction::{InputEnum, Transaction};
use imhamt;
use std::collections::hash_map::DefaultHasher;
use std::convert::Infallible;
use std::sync::Arc;
use tokio::prelude::*;
use tokio::sync::lock::{Lock, LockGuard};

#[derive(Clone)]
pub struct Explorer {
    pub db: ExplorerDB,
    pub schema: Arc<graphql::Schema>,
}

#[derive(Clone)]
pub struct ExplorerDB {
    multiverse: Multiverse<State>,
    longest_chain_tip: Lock<Block>,
    blockchain_config: BlockchainConfig,
}

#[derive(Clone)]
struct BlockchainConfig {
    discrimination: Discrimination,
    consensus_version: ConsensusVersion,
}

type ExplorerBlock = Block;
type Hamt<K, V> = imhamt::Hamt<DefaultHasher, K, V>;

type Transactions = Hamt<FragmentId, HeaderHash>;
type Blocks = Hamt<HeaderHash, ExplorerBlock>;
type ChainLengths = Hamt<ChainLength, HeaderHash>;

type Set<T> = Hamt<T, ()>;

type Addresses = Hamt<Address, Set<FragmentId>>;
type Epochs = Hamt<Epoch, EpochData>;

#[derive(Clone)]
struct State {
    transactions: Transactions,
    blocks: Blocks,
    addresses: Addresses,
    epochs: Epochs,
    chain_lengths: ChainLengths,
}

#[derive(Clone)]
pub struct EpochData {
    first_block: HeaderHash,
    last_block: HeaderHash,
    total_blocks: u32,
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
            ExplorerMsg::NewBlock(block) => info.spawn(explorer_db.apply_block(block).then(
                move |result| match result {
                    Ok(_gc_root) => Ok(()),
                    Err(err) => Err(error!(logger, "Explorer error: {}", err)),
                },
            )),
        }
        future::ok::<(), ()>(())
    }
}

impl ExplorerDB {
    pub fn bootstrap(block0: Block) -> Result<Self> {
        // TODO: Here we should load from Storage to the current Head

        let blocks = apply_block_to_blocks(Blocks::new(), &block0)?;
        let epochs = apply_block_to_epochs(Epochs::new(), &block0)?;
        let chain_lengths = apply_block_to_chain_lengths(ChainLengths::new(), &block0)?;
        let transactions = apply_block_to_transactions(Transactions::new(), &block0)?;

        // TODO: Initialize this
        let addresses = Addresses::new();

        let initial_state = State {
            blocks,
            epochs,
            chain_lengths,
            transactions,
            addresses,
        };

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

        let multiverse = Multiverse::<State>::new();
        // This blocks the thread, but it's only on the node startup when the explorer
        // is enabled
        multiverse
            .insert(block0.chain_length(), block0.id(), initial_state)
            .wait()
            .expect("The multiverse to be empty");

        Ok(Self {
            multiverse,
            longest_chain_tip: Lock::new(block0),
            blockchain_config,
        })
    }

    pub fn apply_block(&mut self, block: Block) -> impl Future<Item = GCRoot, Error = Error> {
        let previous_block = block.header.block_parent_hash();
        let chain_length = block.header.chain_length();
        let block_id = block.header.hash();
        let multiverse = self.multiverse.clone();
        // FIXME: There may be a better way
        let block1 = block.clone();
        let block2 = block.clone();
        multiverse
            .get(*previous_block)
            .map_err(|_: Infallible| unreachable!())
            .and_then(move |maybe_previous_state| {
                let block = block1;
                match maybe_previous_state {
                    Some(state) => {
                        let State {
                            transactions,
                            blocks,
                            addresses,
                            epochs,
                            chain_lengths,
                        } = state;
                        Ok((
                            apply_block_to_transactions(transactions, &block)?,
                            apply_block_to_blocks(blocks, &block)?,
                            apply_block_to_addresses(addresses, &block)?,
                            apply_block_to_epochs(epochs, &block)?,
                            apply_block_to_chain_lengths(chain_lengths, &block)?,
                        ))
                    }
                    None => Err(Error::from(ErrorKind::AncestorNotFound(format!(
                        "{}",
                        block.id()
                    )))),
                }
            })
            .and_then(
                move |(transactions, blocks, addresses, epochs, chain_lengths)| {
                    let chain_length = chain_length.clone();
                    let block_id = block_id.clone();
                    multiverse
                        .insert(
                            chain_length,
                            block_id,
                            State {
                                transactions,
                                blocks,
                                addresses,
                                epochs,
                                chain_lengths,
                            },
                        )
                        .map_err(|_: Infallible| unreachable!())
                },
            )
            .join(
                self.update_longest_chain_tip(block2)
                    .map_err(|_: Infallible| unreachable!()),
            )
            .and_then(|(gc_root, _)| Ok(gc_root))
    }

    fn update_longest_chain_tip(
        &mut self,
        new_block: Block,
    ) -> impl Future<Item = (), Error = Infallible> {
        get_lock(&self.longest_chain_tip).and_then(|mut current| {
            if new_block.header.chain_length() > current.header.chain_length() {
                *current = new_block;
            }
            Ok(())
        })
    }

    pub fn get_block(
        &self,
        block_id: &HeaderHash,
    ) -> impl Future<Item = Option<ExplorerBlock>, Error = Infallible> {
        let multiverse = self.multiverse.clone();
        let block_id = block_id.clone();
        get_lock(&self.longest_chain_tip).and_then(move |tip| {
            multiverse.get((*tip).id()).and_then(move |maybe_state| {
                let state = maybe_state.expect("the longest chain to be indexed");
                Ok(state.blocks.lookup(&block_id).map(|b| (*b).clone()))
            })
        })
    }

    pub fn find_block_by_transaction(
        &self,
        transaction: &FragmentId,
    ) -> impl Future<Item = Option<HeaderHash>, Error = Infallible> {
        let multiverse = self.multiverse.clone();
        let transaction = transaction.clone();
        get_lock(&self.longest_chain_tip).and_then(move |tip| {
            multiverse.get((*tip).id()).and_then(move |maybe_state| {
                let state = maybe_state.expect("the longest chain to be indexed");
                Ok(state.transactions.lookup(&transaction).map(|id| id.clone()))
            })
        })
    }
}

fn get_lock<L>(lock: &Lock<L>) -> impl Future<Item = LockGuard<L>, Error = Infallible> {
    let mut lock = (*lock).clone();
    future::poll_fn(move || Ok(lock.poll_lock()))
}

fn apply_block_to_transactions(transactions: Transactions, block: &Block) -> Result<Transactions> {
    let block_id = block.id();
    let ids = block
        .contents
        .iter()
        .filter(|fragment| is_transaction(fragment))
        .map(|fragment| fragment.id());

    let mut transactions = transactions;
    for id in ids {
        transactions = transactions
            .insert(id, block_id)
            .map_err(|_| ErrorKind::TransactionAlreadyExists(format!("{}", id)))?;
    }

    Ok(transactions)
}

fn apply_block_to_blocks(blocks: Blocks, block: &Block) -> Result<Blocks> {
    let block_id = block.id();
    blocks
        .insert(block_id, (*block).clone())
        .map_err(|_| Error::from(ErrorKind::BlockAlreadyExists(format!("{}", block_id))))
}

fn apply_block_to_addresses(addresses: Addresses, block: &Block) -> Result<Addresses> {
    let mut addresses = addresses;
    let fragments = block.contents.iter();

    for fragment in fragments {
        let fragment_id = fragment.id();
        addresses = match fragment {
            Fragment::Transaction(auth_tx) => {
                apply_transaction_to_addresses(addresses, &fragment_id, &auth_tx.transaction)
            }
            Fragment::OwnerStakeDelegation(auth_tx) => {
                apply_transaction_to_addresses(addresses, &fragment_id, &auth_tx.transaction)
            }
            Fragment::StakeDelegation(auth_tx) => {
                apply_transaction_to_addresses(addresses, &fragment_id, &auth_tx.transaction)
            }
            Fragment::PoolRegistration(auth_tx) => {
                apply_transaction_to_addresses(addresses, &fragment_id, &auth_tx.transaction)
            }
            Fragment::PoolManagement(auth_tx) => {
                apply_transaction_to_addresses(addresses, &fragment_id, &auth_tx.transaction)
            }
            _ => addresses,
        };
    }

    Ok(addresses)
}

fn apply_block_to_epochs(epochs: Epochs, block: &Block) -> Result<Epochs> {
    let epoch_id = block.header.block_date().epoch;
    let block_id = block.id();

    epochs
        .insert_or_update(
            epoch_id,
            EpochData {
                first_block: block_id,
                last_block: block_id,
                total_blocks: 0,
            },
            |data| {
                Ok(Some(EpochData {
                    last_block: block_id,
                    total_blocks: data.total_blocks + 1,
                    ..*data
                }))
            },
        )
        .map_err(|_: imhamt::InsertOrUpdateError<Infallible>| {
            unreachable!();
        })
}

fn apply_block_to_chain_lengths(
    chain_lengths: ChainLengths,
    block: &Block,
) -> Result<ChainLengths> {
    let new_block_chain_length = block.chain_length();
    let new_block_hash = block.id();
    chain_lengths
        .insert(new_block_chain_length, new_block_hash)
        .map_err(|_| {
            // I think this shouldn't happen
            Error::from(ErrorKind::ChainLengthBlockAlreadyExists(u32::from(
                new_block_chain_length,
            )))
        })
}

fn is_transaction(fragment: &Fragment) -> bool {
    // XXX: maybe this shouldn't be here? and perhaps adding all the cases explicitly is better
    match fragment {
        Fragment::Transaction(_) => true,
        Fragment::OwnerStakeDelegation(_) => true,
        Fragment::StakeDelegation(_) => true,
        Fragment::PoolRegistration(_) => true,
        Fragment::PoolManagement(_) => true,
        _ => false,
    }
}

fn apply_transaction_to_addresses<T>(
    addresses: Addresses,
    txid: &FragmentId,
    tx: &Transaction<Address, T>,
) -> Addresses {
    let outputs = &tx.outputs;
    let inputs = &tx.inputs;

    let mut addresses = addresses;

    for output in outputs {
        addresses = addresses
            .insert_or_update(
                output.address.clone(),
                Set::new().insert(txid.clone(), ()).unwrap(),
                |set| {
                    if set.contains_key(txid) {
                        Ok(Some(set.clone()))
                    } else {
                        Ok::<Option<Set<FragmentId>>, Infallible>(Some(
                            set.insert(txid.clone(), ())
                                .expect("the address to not be in the set"),
                        ))
                    }
                },
            )
            // This shouldn't happen
            .unwrap();
    }

    for input in inputs {
        match input.to_enum() {
            InputEnum::AccountInput(_id, _value) => {
                // TODO: How do I get an Address from an AccountIdentifier?
                // Can I do it without knowing the Discrimination?
                ();
            }
            InputEnum::UtxoInput(_) => {
                //TODO: Resolve utxos
                ();
            }
        }
    }

    addresses
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

        BlockchainConfig {
            discrimination,
            consensus_version,
        }
    }
}
