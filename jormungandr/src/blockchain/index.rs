use crate::blockchain::{
    chain::MAIN_BRANCH_TAG,
    storage::{Storage, StorageError},
};
use chain_impl_mockchain::header::{BlockDate, ChainLength, Header, HeaderId};
use std::collections::BTreeMap;
use thiserror::Error;
use tokio::{prelude::*, sync::lock::Lock};

#[derive(Clone)]
pub struct Index {
    inner: Lock<IndexInternal>,
}

#[derive(Default)]
struct IndexInternal {
    chain_length_index: BTreeMap<ChainLength, HeaderId>,
    block_date_index: BTreeMap<BlockDate, HeaderId>,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("no blockchain head found in the database")]
    NoHeadFound,
    #[error("storage error")]
    StorageError(#[from] StorageError),
}

pub enum IndexRequest {
    ChainLength(ChainLength),
    BlockDate(BlockDate),
}

impl Index {
    pub fn new() -> Self {
        Self {
            inner: Lock::new(Default::default()),
        }
    }

    pub fn get(&self, key: IndexRequest) -> impl Future<Item = Option<HeaderId>, Error = ()> {
        let mut inner = self.inner.clone();
        future::poll_fn(move || Ok(inner.poll_lock())).map(move |index| {
            match key {
                IndexRequest::ChainLength(key) => index.chain_length_index.get(&key),
                IndexRequest::BlockDate(key) => index.block_date_index.get(&key),
            }
            .map(|v| v.to_owned())
        })
    }

    pub fn update_from_storage(
        &mut self,
        storage: Storage,
    ) -> impl Future<Item = (), Error = Error> {
        let mut inner = self.inner.clone();
        let mut inner_2 = self.inner.clone();

        future::poll_fn(move || Ok(inner.poll_lock())).and_then(move |index| {
            // stream of values from the storage (HEAD to genesis)
            let mut storage_stream = storage
                .get_tag(MAIN_BRANCH_TAG.to_owned())
                .map_err(Error::StorageError)
                .map(|r| r.ok_or(Error::NoHeadFound))
                .flatten()
                .and_then(move |hash| {
                    storage
                        .stream_from_to_reversed(hash, None)
                        .map_err(Error::StorageError)
                })
                .map(|stream| stream.map_err(Error::StorageError))
                .into_stream()
                .flatten();

            // interrupt the stream of blocks once we met an existing block
            stream::poll_fn(move || match storage_stream.poll()? {
                Async::Ready(Some(block)) => {
                    let id = block.header.id().clone();
                    let by_chain_length = header_to_chain_length(&block.header);
                    let by_block_date = header_to_block_date(&block.header);
                    // we only check one of arguments assuming that indexes are
                    // under the same lock and thus are always consistent
                    if let Some(stored_id) = index.chain_length_index.get(&by_chain_length) {
                        if stored_id == &id {
                            return Ok(Async::Ready(None));
                        }
                    }
                    Ok(Async::Ready(Some((by_chain_length, by_block_date, id))))
                }
                Async::Ready(None) => Ok(Async::Ready(None)),
                Async::NotReady => Ok(Async::NotReady),
            })
            // fuse will ensure that the stream is stopped after the first None
            .fuse()
            .collect()
            .and_then(move |new_keys| {
                future::poll_fn(move || Ok(inner_2.poll_lock())).map(move |mut index| {
                    if new_keys.is_empty() {
                        return;
                    }

                    clean_index(&mut index.chain_length_index, &new_keys[0].0);
                    clean_index(&mut index.block_date_index, &new_keys[0].1);

                    new_keys
                        .into_iter()
                        .for_each(|(chain_length, block_date, id)| {
                            index.chain_length_index.insert(chain_length, id.clone());
                            index.block_date_index.insert(block_date, id);
                        });
                })
            })
        })
    }
}

fn header_to_chain_length(h: &Header) -> ChainLength {
    h.chain_length()
}

fn header_to_block_date(h: &Header) -> BlockDate {
    h.block_date()
}

fn clean_index<T>(index: &mut BTreeMap<T, HeaderId>, key: &T)
where
    T: Clone + Ord,
{
    use std::ops::Bound;

    let to_remove: Vec<_> = index
        .range((Bound::Excluded(key), Bound::Unbounded))
        .map(|(key, _)| key)
        .cloned()
        .collect();

    to_remove.into_iter().for_each(|key| {
        index.remove(&key);
    });
}
