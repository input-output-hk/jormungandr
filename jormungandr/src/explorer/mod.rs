pub mod error;
pub mod graphql;

use self::error::{Error, ErrorKind, Result};
use self::graphql::Context;
use super::blockchain::Blockchain;
use crate::blockcfg::{Block, ChainLength, Epoch, Fragment, FragmentId, Header, HeaderHash};
use crate::blockchain::Multiverse;
use crate::intercom::ExplorerMsg;
use crate::utils::task::{Input, TokioServiceInfo};
use chain_addr::Address;
use chain_core::property::Block as _;
use chain_core::property::Fragment as _;
use chain_impl_mockchain::fee::LinearFee;
use chain_impl_mockchain::multiverse::GCRoot;
use imhamt;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashSet;
use std::convert::Infallible;
use std::sync::Arc;
use tokio::prelude::*;
use tokio::sync::lock::Lock;

#[derive(Clone)]
pub struct Explorer {
    pub db: ExplorerDB,
    pub schema: Arc<graphql::Schema>,
    pub blockchain: Blockchain,
}

#[derive(Clone)]
pub struct ExplorerDB {
    multiverse: Multiverse<State>,
    longest_chain_tip: Lock<Option<Block>>,
}

type ExplorerBlock = Block;
type Hamt<K, V> = imhamt::Hamt<DefaultHasher, K, V>;

type Transactions = Hamt<FragmentId, HeaderHash>;
type Blocks = Hamt<HeaderHash, ExplorerBlock>;
type ChainLengths = Hamt<ChainLength, HeaderHash>;
// FIXME: Probably using a Hamt<FragmentId, ()> would be better than HashSet?
type Addresses = Hamt<Address, HashSet<FragmentId>>;
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
    pub fn new(db: ExplorerDB, schema: graphql::Schema, blockchain: Blockchain) -> Explorer {
        Explorer {
            db,
            schema: Arc::new(schema),
            blockchain,
        }
    }

    pub fn context(&self) -> Context {
        Context {
            db: self.db.clone(),
            blockchain: self.blockchain.clone(),
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
        let blockchain = self.blockchain.clone();
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
    pub fn new() -> Self {
        Self {
            multiverse: Multiverse::<State>::new(),
            longest_chain_tip: Lock::new(None),
        }
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
        get_lock(&self.longest_chain_tip).and_then(|mut guard| {
            match *guard {
                Some(ref current) => {
                    if new_block.header.chain_length() > current.header.chain_length() {
                        *guard = Some(new_block);
                    }
                }
                None => {
                    *guard = Some(new_block);
                }
            };
            Ok(())
        })
    }

    pub fn is_block_in_explorer(
        &self,
        hash: HeaderHash,
    ) -> impl Future<Item = Option<Header>, Error = Infallible> {
        //XXX: Probably the clone is not necessary
        self.multiverse
            .get(hash)
            .map(|state_option| state_option.is_some())
    }

    pub fn find_block_by_transaction(
        &self,
        transaction: FragmentId,
    ) -> impl Future<Item = Option<ExplorerBlock>, Error = Infallible> {
        let multiverse = self.multiverse.clone();
        get_lock(&self.longest_chain_tip).and_then(move |maybe_tip| {
            let tip = match *maybe_tip {
                Some(ref tip) => tip.id(),
                None => return future::Either::A(Ok(None).into_future()),
            };

            future::Either::B(multiverse.get(tip).and_then(move |maybe_state| {
                let state = match maybe_state {
                    Some(state) => state,
                    None => return unreachable!(),
                };

                Ok(state
                    .transactions
                    .lookup(&transaction)
                    .and_then(|block_id| state.blocks.lookup(&block_id).map(|b| (*b).clone())))
            }))
        })
    }

    pub fn get_header(
        &self,
        hash: HeaderHash,
    ) -> impl Future<Item = Option<Header>, Error = Infallible> {
        unimplemented!();
        // Just to make it compile
        Ok(None).into_future()
    }

    pub fn get_next_block(
        &self,
        block_id: HeaderHash,
    ) -> impl Future<Item = Option<HeaderHash>, Error = Infallible> {
        unimplemented!();
        // Just to make it compile
        Ok(None).into_future()
    }

    pub fn get_epoch_data(
        &self,
        epoch: Epoch,
    ) -> impl Future<Item = Option<EpochData>, Error = Infallible> {
        unimplemented!();
        // Just to make it compile
        Ok(None).into_future()
    }

    pub fn get_current_status(&self) -> impl Future<Item = (), Error = Infallible> {
        unimplemented!();
        // Just to make it compile
        Ok(()).into_future()
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
    // FIXME: Probably there is a cleaner way to do this
    let txs = block.contents.iter().filter_map(|fragment| match fragment {
        Fragment::Transaction(auth_tx) => Some((
            fragment.id(),
            &auth_tx.transaction.inputs,
            &auth_tx.transaction.outputs,
        )),
        Fragment::OwnerStakeDelegation(auth_tx) => Some((
            fragment.id(),
            &auth_tx.transaction.inputs,
            &auth_tx.transaction.outputs,
        )),
        Fragment::StakeDelegation(auth_tx) => Some((
            fragment.id(),
            &auth_tx.transaction.inputs,
            &auth_tx.transaction.outputs,
        )),
        Fragment::PoolRegistration(auth_tx) => Some((
            fragment.id(),
            &auth_tx.transaction.inputs,
            &auth_tx.transaction.outputs,
        )),
        Fragment::PoolManagement(auth_tx) => Some((
            fragment.id(),
            &auth_tx.transaction.inputs,
            &auth_tx.transaction.outputs,
        )),
        _ => None,
    });

    let mut addresses = addresses;

    for (txid, inputs, outputs) in txs {
        for output in outputs {
            addresses = addresses
                .insert_or_update(
                    output.address.clone(),
                    [txid].iter().cloned().collect(),
                    |set| {
                        let mut new_set: HashSet<FragmentId> = set.iter().cloned().collect();
                        new_set.insert(txid);
                        std::result::Result::<_, Infallible>::Ok(Some(new_set))
                    },
                )
                // FIXME: I still don't know when could this happen
                .unwrap();
        }
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
            // I'm not sure in which case could this happen
            unimplemented!();
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
