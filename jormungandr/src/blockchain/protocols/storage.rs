use crate::{
    blockcfg::{Block, HeaderHash},
    start_up::NodeStorage,
};
use chain_storage::error::Error as StorageError;
use tokio::prelude::*;
use tokio::sync::lock::Lock;

#[derive(Clone)]
pub struct Storage {
    inner: Lock<NodeStorage>,
}

impl Storage {
    pub fn new(storage: NodeStorage) -> Self {
        Storage {
            inner: Lock::new(storage),
        }
    }

    pub fn get(
        &self,
        header_hash: HeaderHash,
    ) -> impl Future<Item = Option<Block>, Error = StorageError> {
        let mut inner = self.inner.clone();

        future::poll_fn(move || Ok(inner.poll_lock())).and_then(move |guard| {
            match guard.get_block(&header_hash) {
                Err(StorageError::BlockNotFound) => future::ok(None),
                Err(error) => future::err(error),
                Ok((block, _block_info)) => future::ok(Some(block)),
            }
        })
    }

    pub fn block_exists(
        &self,
        header_hash: HeaderHash,
    ) -> impl Future<Item = bool, Error = StorageError> {
        let mut inner = self.inner.clone();

        future::poll_fn(move || Ok(inner.poll_lock())).and_then(move |guard| {
            match guard.block_exists(&header_hash) {
                Err(StorageError::BlockNotFound) => future::ok(false),
                Err(error) => future::err(error),
                Ok(existence) => future::ok(existence),
            }
        })
    }

    pub fn put_block(&mut self, block: Block) -> impl Future<Item = (), Error = StorageError> {
        let mut inner = self.inner.clone();

        future::poll_fn(move || Ok(inner.poll_lock())).and_then(move |mut guard| {
            match guard.put_block(&block) {
                Err(StorageError::BlockNotFound) => unreachable!(),
                Err(error) => future::err(error),
                Ok(()) => future::ok(()),
            }
        })
    }
}
