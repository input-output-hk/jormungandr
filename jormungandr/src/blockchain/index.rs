use crate::blockchain::{
    chain::MAIN_BRANCH_TAG,
    storage::{Storage, StorageError},
};
use chain_impl_mockchain::header::{Header, HeaderId};
use std::collections::BTreeMap;
use thiserror::Error;
use tokio::{prelude::*, sync::lock::Lock};

type HeaderToKey<K> = fn(Header) -> K;

pub struct Index<K>
where
    K: std::cmp::Ord,
{
    transform: HeaderToKey<K>,
    index_lock: Lock<BTreeMap<K, HeaderId>>,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("no blockchain head found in the database")]
    NoHeadFound,
    #[error("storage error")]
    StorageError(#[from] StorageError),
}

impl<K> Index<K>
where
    K: std::cmp::Ord + Clone,
{
    pub fn new(transform: HeaderToKey<K>) -> Self {
        Self {
            transform,
            index_lock: Lock::new(BTreeMap::new()),
        }
    }

    pub fn get(&self, key: K) -> impl Future<Item = Option<HeaderId>, Error = ()> {
        let mut index_lock = self.index_lock.clone();
        future::poll_fn(move || Ok(index_lock.poll_lock()))
            .map(move |index| index.get(&key).map(|v| v.to_owned()))
    }

    pub fn update_from_storage(
        &mut self,
        storage: Storage,
    ) -> impl Future<Item = (), Error = Error> {
        let mut index_lock = self.index_lock.clone();
        let mut index_lock_2 = self.index_lock.clone();
        let transform = self.transform;

        future::poll_fn(move || Ok(index_lock.poll_lock())).and_then(move |mut index| {
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
                    let key = (transform)(block.header);
                    if let Some(stored_id) = index.get(&key) {
                        if stored_id == &id {
                            return Ok(Async::Ready(None));
                        }
                    }
                    Ok(Async::Ready(Some((key, id))))
                }
                Async::Ready(None) => Ok(Async::Ready(None)),
                Async::NotReady => Ok(Async::NotReady),
            })
            // fuse will ensure that the stream is stopped after the first None
            .fuse()
            .collect()
            .and_then(move |new_keys| {
                future::poll_fn(move || Ok(index_lock_2.poll_lock())).map(move |mut index| {
                    use std::ops::Bound;
                    if new_keys.is_empty() {
                        return;
                    }
                    let to_remove: Vec<_> = index
                        .range((Bound::Excluded(&new_keys[0].0), Bound::Unbounded))
                        .map(|(key, _)| key)
                        .cloned()
                        .collect();
                    to_remove.into_iter().for_each(|key| {
                        index.remove(&key);
                    });
                    new_keys.into_iter().for_each(|(key, id)| {
                        index.insert(key, id);
                    });
                })
            })
        })
    }
}
