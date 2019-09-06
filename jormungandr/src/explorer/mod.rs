pub mod graphql;

use self::graphql::Context;
use super::blockchain::{Blockchain, Ref};
use crate::blockcfg::{ChainLength, FragmentId, Header, HeaderHash};
use crate::blockchain::Multiverse;
use crate::intercom::ExplorerMsg;
use crate::utils::task::{Input, TokioServiceInfo};
use chain_core::property::Fragment;
use chain_impl_mockchain::multiverse::GCRoot;
use chain_storage::error::Error as StorageError;
use futures::lazy;
use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::Arc;
use tokio::prelude::*;
use tokio::sync::lock::Lock;

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
        Self {
            multiverse: Multiverse::<Ref>::new(),
            chain_length_to_hash: Lock::new(HashMap::new()),
            transaction_to_block: Lock::new(HashMap::new()),
        }
    }

    pub fn store_ref(
        &mut self,
        new_block_ref: Ref,
    ) -> impl Future<Item = GCRoot, Error = Infallible> {
        let chain_length = new_block_ref.chain_length();
        let header_hash = new_block_ref.hash();

        // Clone things to move into closures, this is just cloning locks
        let mut map = self.chain_length_to_hash.clone();
        let multiverse = self.multiverse.clone();

        future::poll_fn(move || Ok(map.poll_lock()))
            // Store in chain_length_to_hash
            .map(move |mut guard| {
                guard
                    .entry(chain_length)
                    .or_insert(Vec::new())
                    .push(new_block_ref.hash());
                new_block_ref
            })
            // Store in the multiverse
            .and_then(move |inserted_ref| {
                multiverse.insert(chain_length, header_hash, inserted_ref)
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

    pub fn get_header(
        &self,
        hash: HeaderHash,
    ) -> impl Future<Item = Option<Header>, Error = Infallible> {
        //XXX: Probably the clone is not necessary
        self.multiverse
            .get(hash)
            .map(|maybe_block_ref| maybe_block_ref.map(|r| (*r.header()).clone()))
    }

    pub fn find_block_by_transaction(
        &self,
        transaction: FragmentId,
    ) -> impl Future<Item = Option<HeaderHash>, Error = StorageError> {
        let mut blocks = self.transaction_to_block.clone();
        future::poll_fn(move || Ok(blocks.poll_lock()))
            .and_then(move |guard| Ok(guard.get(&transaction).map(|h| (*h).clone())))
    }
}
