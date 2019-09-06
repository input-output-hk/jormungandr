pub mod graphql;

use self::graphql::Context;
use super::blockchain::{Blockchain, Ref};
use crate::blockcfg::{ChainLength, Epoch, FragmentId, Header, HeaderHash};
use crate::blockchain::Multiverse;
use crate::intercom::ExplorerMsg;
use crate::utils::task::{Input, TokioServiceInfo};
use chain_core::property::Fragment;
use chain_impl_mockchain::fee::LinearFee;
use chain_impl_mockchain::multiverse::GCRoot;
use chain_storage::error::Error as StorageError;
use futures::lazy;
use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::Arc;
use tokio::prelude::*;
use tokio::sync::lock::{Lock, LockGuard};

error_chain! {
    foreign_links {
        StorageError(StorageError);
    }
    errors {
        BlockNotFound(hash: String) {
            description("block not found"),
            display("block '{}' cannot be found in the explorer", hash)
        }
    }
}

#[derive(Clone)]
pub struct Explorer {
    pub db: ExplorerDB,
    pub schema: Arc<graphql::Schema>,
    pub blockchain: Blockchain,
}

#[derive(Clone)]
pub struct ExplorerDB {
    multiverse: Multiverse<Ref>,
    // XXX: A better locking strategy could be better, as locking the entire hashmaps
    // is probably too much.
    chain_length_to_hash: Lock<HashMap<ChainLength, Vec<HeaderHash>>>,
    transaction_to_block: Lock<HashMap<FragmentId, HeaderHash>>,
    epochs: Lock<HashMap<Epoch, EpochData>>,
    next_block: Lock<HashMap<HeaderHash, HeaderHash>>,
    status: Lock<Status>,
}

#[derive(Clone)]
pub struct EpochData {
    first_block: HeaderHash,
    last_block: HeaderHash,
    total_blocks: u32,
    fees: LinearFee,
}

#[derive(Clone)]
pub struct Status {
    current_epoch: Epoch,
    // FIXME: This is an Option because the current initialization is a dummy one
    latest_block: Option<HeaderHash>,
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
            ExplorerMsg::NewBlock(new_block_ref) => info.spawn(lazy(move || {
                explorer_db
                    .store_ref(new_block_ref.clone())
                    .map_err(|_| unreachable!())
                    .join(
                        explorer_db
                            .index_transactions(new_block_ref, blockchain)
                            .map_err(move |err| {
                                error!(logger, "Explorer error: {}", err);
                            }),
                    )
                    .map(move |_| ())
            })),
        }
        future::ok::<(), ()>(())
    }
}

impl ExplorerDB {
    pub fn new() -> Self {
        // TODO: Some kind of recovery/initialization from Storage
        Self {
            multiverse: Multiverse::<Ref>::new(),
            chain_length_to_hash: Lock::new(HashMap::new()),
            transaction_to_block: Lock::new(HashMap::new()),
            epochs: Lock::new(HashMap::new()),
            next_block: Lock::new(HashMap::new()),
            status: Lock::new(Status {
                // TODO: Get this values from Storage or some place (e.g: explorer persistance)
                // this is just a Dummy initialization
                current_epoch: 0,
                latest_block: None,
            }),
        }
    }

    pub fn store_ref(
        &mut self,
        new_block_ref: Ref,
    ) -> impl Future<Item = GCRoot, Error = Infallible> {
        let chain_length = new_block_ref.chain_length();
        let header_hash = new_block_ref.hash();

        let multiverse = self.multiverse.clone();

        self.index_chain_length(&new_block_ref)
            .join(self.index_next_block(&new_block_ref))
            .join(self.index_epoch(&new_block_ref))
            .join(self.update_status(&new_block_ref))
            // Insert in multiverse
            .join(multiverse.insert(chain_length, header_hash, new_block_ref.clone()))
            .map(|(_, gcroot)| gcroot)
    }

    fn update_status(&mut self, new_block_ref: &Ref) -> impl Future<Item = (), Error = Infallible> {
        let current_epoch = new_block_ref.block_date().epoch;
        let latest_block = new_block_ref.hash();
        get_lock(&self.status).and_then(move |mut guard| {
            guard.current_epoch = current_epoch;
            guard.latest_block = Some(latest_block);
            Ok(())
        })
    }

    fn index_chain_length(
        &mut self,
        new_block_ref: &Ref,
    ) -> impl Future<Item = (), Error = Infallible> {
        let header_hash = new_block_ref.hash();
        let chain_length = new_block_ref.chain_length();

        get_lock(&self.chain_length_to_hash).and_then(move |mut guard| {
            guard
                .entry(chain_length)
                .or_insert(Vec::new())
                .push(header_hash);

            std::result::Result::<(), Infallible>::Ok(())
        })
    }

    fn index_next_block(
        &mut self,
        new_block_ref: &Ref,
    ) -> impl Future<Item = (), Error = Infallible> {
        let parent_hash = (*new_block_ref.block_parent_hash()).clone();
        let hash = new_block_ref.hash();
        get_lock(&self.next_block).and_then(move |mut map| {
            map.insert(parent_hash, hash);

            std::result::Result::<(), Infallible>::Ok(())
        })
    }

    fn index_transactions(
        &mut self,
        new_block_ref: Ref,
        blockchain: Blockchain,
    ) -> impl Future<Item = (), Error = Error> {
        let mut map = self.transaction_to_block.clone();
        blockchain
            .storage()
            .get(new_block_ref.hash())
            .map_err(|err| ErrorKind::StorageError(err).into())
            .join(future::poll_fn(move || Ok(map.poll_lock())))
            .and_then(move |(block, mut guard)| {
                if let Some(b) = block {
                    for fragment in b.contents.iter() {
                        guard.insert(fragment.id(), new_block_ref.hash());
                    }
                } else {
                    return future::err(
                        ErrorKind::BlockNotFound(new_block_ref.hash().to_string()).into(),
                    );
                }
                future::ok(())
            })
    }

    fn index_epoch(&mut self, new_block_ref: &Ref) -> impl Future<Item = (), Error = Infallible> {
        let hash = new_block_ref.hash();
        let fees = new_block_ref.epoch_ledger_parameters().fees;
        let epoch = new_block_ref.block_date().epoch;
        get_lock(&self.epochs).and_then(move |mut guard| {
            guard
                .entry(epoch)
                .and_modify(|data| {
                    data.last_block = hash;
                    data.total_blocks = data.total_blocks + 1;
                })
                .or_insert(EpochData {
                    first_block: hash,
                    last_block: hash,
                    total_blocks: 1,
                    fees,
                });
            std::result::Result::<(), Infallible>::Ok(())
        })
    }

    pub fn is_block_in_explorer(
        &self,
        hash: HeaderHash,
    ) -> impl Future<Item = bool, Error = Infallible> {
        self.multiverse
            .get(hash)
            .map(|ref_option| ref_option.is_some())
    }

    pub fn is_epoch_in_explorer(
        &self,
        epoch_number: Epoch,
    ) -> impl Future<Item = bool, Error = Infallible> {
        get_lock(&self.epochs).map(move |guard| guard.get(&epoch_number).is_some())
    }

    pub fn find_block_by_transaction(
        &self,
        transaction: FragmentId,
    ) -> impl Future<Item = Option<HeaderHash>, Error = Infallible> {
        let mut blocks = self.transaction_to_block.clone();
        future::poll_fn(move || Ok(blocks.poll_lock()))
            .and_then(move |guard| Ok(guard.get(&transaction).map(|h| (*h).clone())))
    }

    pub fn get_header(
        &self,
        hash: HeaderHash,
    ) -> impl Future<Item = Option<Header>, Error = Infallible> {
        //XXX: Probably the clone is not necessary
        self.multiverse
            .get(hash)
            .map(|maybe_block_ref| maybe_block_ref.map(|r| (*r.header()).clone()))
    }

    pub fn get_next_block(
        &self,
        block_id: HeaderHash,
    ) -> impl Future<Item = Option<HeaderHash>, Error = Infallible> {
        get_lock(&self.next_block).map(move |guard| guard.get(&block_id).map(|h| (*h).clone()))
    }

    pub fn get_epoch_data(
        &self,
        epoch: Epoch,
    ) -> impl Future<Item = Option<EpochData>, Error = Infallible> {
        get_lock(&self.epochs).map(move |guard| guard.get(&epoch).map(|h| (*h).clone()))
    }

    pub fn get_current_status(&self) -> impl Future<Item = Status, Error = Infallible> {
        get_lock(&self.status).map(|guard| (*guard).clone())
    }
}

fn get_lock<L>(lock: &Lock<L>) -> impl Future<Item = LockGuard<L>, Error = Infallible> {
    let mut lock = (*lock).clone();
    future::poll_fn(move || Ok(lock.poll_lock()))
}
