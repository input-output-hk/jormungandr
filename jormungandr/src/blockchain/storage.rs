use crate::{
    blockcfg::{Block, HeaderHash},
    start_up::{NodeStorage, NodeStorageConnection},
};
use async_trait::async_trait;
use bb8::{ManageConnection, Pool, PooledConnection};
use chain_storage::store::{for_path_to_nth_ancestor, BlockInfo, BlockStore};
use futures03::{
    future,
    prelude::*,
    sink::{Sink, SinkExt},
    stream::{self, Stream},
};
use std::{pin::Pin, sync::Arc};
use tokio_02::{sync::Mutex, task::spawn_blocking};

pub use chain_storage::error::Error as StorageError;

#[derive(Clone)]
struct ConnectionManager {
    inner: Arc<NodeStorage>,
}

impl ConnectionManager {
    pub fn new(storage: NodeStorage) -> Self {
        Self {
            inner: Arc::new(storage),
        }
    }
}

#[async_trait]
impl ManageConnection for ConnectionManager {
    type Connection = NodeStorageConnection;
    type Error = StorageError;

    async fn connect(&self) -> Result<Self::Connection, Self::Error> {
        let inner = self.inner.clone();
        spawn_blocking(move || inner.connect())
            .await
            .map_err(|e| StorageError::BackendError(Box::new(e)))
            .and_then(std::convert::identity)
    }

    async fn is_valid(&self, conn: Self::Connection) -> Result<Self::Connection, Self::Error> {
        spawn_blocking(move || conn.ping().and(Ok(conn)))
            .await
            .map_err(|e| StorageError::BackendError(Box::new(e)))
            .and_then(std::convert::identity)
    }

    fn has_broken(&self, conn: &mut Self::Connection) -> bool {
        conn.ping().is_ok()
    }
}

#[derive(Clone)]
pub struct Storage {
    pool: Pool<ConnectionManager>,

    // All write operations must be performed only via this lock. The lock helps
    // us to ensure that all of the write operations are performed in the right
    // sequence. Otherwise they can be performed out of the expected order (for
    // example, by different tokio executors) which eventually leads to a panic
    // because the block data would be inconsistent at the time of a write.
    write_connection_lock: Arc<Mutex<NodeStorageConnection>>,
}

pub struct Ancestor {
    pub header_hash: HeaderHash,
    pub distance: u64,
}

struct BlockIterState {
    to_depth: u64,
    cur_depth: u64,
    pending_infos: Vec<BlockInfo<HeaderHash>>,
}

impl Storage {
    pub async fn new(storage: NodeStorage) -> Self {
        let manager = ConnectionManager::new(storage);
        let pool = Pool::builder().build(manager).await.unwrap();
        let write_connection_lock =
            Arc::new(Mutex::new(pool.dedicated_connection().await.unwrap()));

        Storage {
            pool,
            write_connection_lock,
        }
    }

    async fn run<F, R>(&self, f: F) -> Result<R, StorageError>
    where
        F: FnOnce(PooledConnection<'_, ConnectionManager>) -> Result<R, StorageError>
            + Send
            + 'static,
        R: Send + 'static,
    {
        let connection = self
            .pool
            .clone()
            .get()
            .await
            .map_err(|e| StorageError::BackendError(Box::new(e)))?;

        spawn_blocking(move || f(connection))
            .await
            .map_err(|e| StorageError::BackendError(Box::new(e)))
            .and_then(std::convert::identity)
    }

    pub async fn get_tag(&self, tag: String) -> Result<Option<HeaderHash>, StorageError> {
        self.run(|connection| connection.get_tag(&tag)).await
    }

    pub async fn put_tag(&self, tag: String, header_hash: HeaderHash) -> Result<(), StorageError> {
        let guard = self.write_connection_lock.clone().lock().await;
        spawn_blocking(move || guard.put_tag(&tag, &header_hash))
            .await
            .map_err(|e| StorageError::BackendError(Box::new(e)))
            .and_then(std::convert::identity)
    }

    pub async fn get(&self, header_hash: HeaderHash) -> Result<Option<Block>, StorageError> {
        self.run(|connection| match connection.get_block(&header_hash) {
            Err(StorageError::BlockNotFound) => Ok(None),
            Ok((block, _block_info)) => Ok(Some(block)),
            Err(e) => Err(e),
        })
        .await
    }

    pub async fn get_with_info(
        &self,
        header_hash: HeaderHash,
    ) -> Result<Option<(Block, BlockInfo<HeaderHash>)>, StorageError> {
        self.run(|connection| match connection.get_block(&header_hash) {
            Err(StorageError::BlockNotFound) => Ok(None),
            Ok(v) => Ok(Some(v)),
            Err(e) => Err(e),
        })
        .await
    }

    pub async fn block_exists(&self, header_hash: HeaderHash) -> Result<bool, StorageError> {
        self.run(|connection| match connection.block_exists(&header_hash) {
            Err(StorageError::BlockNotFound) => Ok(false),
            Ok(r) => Ok(r),
            Err(e) => Err(e),
        })
        .await
    }

    pub async fn put_block(&self, block: Block) -> Result<(), StorageError> {
        let guard = self.write_connection_lock.clone().lock().await;
        spawn_blocking(move || match guard.put_block(&block) {
            Err(StorageError::BlockNotFound) => unreachable!(),
            Err(e) => Err(e),
            Ok(()) => Ok(()),
        })
        .await
        .map_err(|e| StorageError::BackendError(Box::new(e)))
        .and_then(std::convert::identity)
    }

    /// Return values:
    /// - `Ok(stream)` - `from` is ancestor of `to`, returns blocks between them
    /// - `Err(CannotIterate)` - `from` is not ancestor of `to`
    /// - `Err(BlockNotFound)` - `from` or `to` was not found
    /// - `Err(_)` - some other storage error
    pub async fn stream_from_to(
        &self,
        from: HeaderHash,
        to: HeaderHash,
    ) -> Result<impl Stream<Item = Result<Block, StorageError>>, StorageError> {
        let init_state = self
            .run(|connection| match connection.is_ancestor(&from, &to) {
                Ok(Some(distance)) => match connection.get_block_info(&to) {
                    Ok(to_info) => Ok(BlockIterState::new(to_info, distance)),
                    Err(e) => Err(e),
                },
                Ok(None) => Err(StorageError::CannotIterate),
                Err(e) => Err(e),
            })
            .await?;

        let pool = self.pool.clone();

        Ok(stream::unfold(init_state, |state| {
            async move {
                if !state.has_next() {
                    return None;
                }
                let res = state.get_next(pool).await;
                Some((res, state))
            }
        }))
    }

    /// Stream a branch ending at `to` and starting from the ancestor
    /// at `depth` or at the first ancestor since genesis block
    /// if `depth` is given as `None`.
    ///
    /// This function uses buffering in the sink to reduce lock contention.
    pub async fn send_branch<S, E>(
        &self,
        to: HeaderHash,
        depth: Option<u64>,
        sink: Pin<Box<S>>,
    ) -> Result<(), S::Error>
    where
        S: Sink<Result<Block, E>, Error = E>,
        E: From<StorageError>,
    {
        let res = self
            .run(|connection| {
                connection.get_block_info(&to).map(|to_info| {
                    let depth = depth.unwrap_or(to_info.depth - 1);
                    BlockIterState::new(to_info, depth)
                })
            })
            .await;

        match res {
            Ok(iter) => {
                let mut state = SendState {
                    sink,
                    iter,
                    pending: None,
                };

                while state.r#continue().await? {
                    state.fill_sink(self.pool.clone()).await?;
                }
            }
            Err(e) => {
                sink.send_all(&mut stream::once(Box::pin(async { Err(e.into()) })))
                    .await?;
            }
        }

        Ok(())
    }

    pub async fn find_closest_ancestor(
        &self,
        checkpoints: Vec<HeaderHash>,
        descendant: HeaderHash,
    ) -> Result<Option<Ancestor>, StorageError> {
        self.run(|connection| {
            let mut ancestor = None;
            let mut closest_found = std::u64::MAX;
            for checkpoint in checkpoints {
                // Checkpoints sent by a peer may not
                // be present locally, so we need to ignore certain errors
                match connection.is_ancestor(&checkpoint, &descendant) {
                    Ok(None) => {}
                    Ok(Some(distance)) => {
                        if closest_found > distance {
                            ancestor = Some(checkpoint);
                            closest_found = distance;
                        }
                    }
                    Err(e) => {
                        // Checkpoints sent by a peer may not
                        // be present locally, so we need to ignore certain errors
                        match e {
                            StorageError::BlockNotFound => {
                                // FIXME: add block hash into the error so we
                                // can see which of the two it is.
                                // For now, just ignore either.
                            }
                            _ => return Err(e),
                        }
                    }
                }
            }
            Ok(ancestor.map(|header_hash| Ancestor {
                header_hash,
                distance: closest_found,
            }))
        })
        .await
    }
}

impl BlockIterState {
    fn new(to_info: BlockInfo<HeaderHash>, distance: u64) -> Self {
        BlockIterState {
            to_depth: to_info.depth,
            cur_depth: to_info.depth - distance,
            pending_infos: vec![to_info],
        }
    }

    fn has_next(&self) -> bool {
        self.cur_depth < self.to_depth
    }

    async fn get_next(&mut self, pool: Pool<ConnectionManager>) -> Result<Block, StorageError> {
        assert!(self.has_next());

        let store = pool
            .clone()
            .get()
            .await
            .map_err(|e| StorageError::BackendError(Box::new(e)))?;

        self.cur_depth += 1;

        let block_info = self.pending_infos.pop().unwrap();

        let cur_depth = self.cur_depth;

        let (mut pending_infos, block) = spawn_blocking(move || {
            if block_info.depth == cur_depth {
                // We've seen this block on a previous ancestor traversal.
                let (block, _block_info) = store.get_block(&block_info.block_hash)?;
                Ok((Vec::new(), block))
            } else {
                // We don't have this block yet, so search back from
                // the furthest block that we do have.
                assert!(cur_depth < block_info.depth);
                let depth = block_info.depth;
                let parent = block_info.parent_id();
                let mut pending_infos = Vec::new();
                pending_infos.push(block_info);
                let block_info = for_path_to_nth_ancestor(
                    &*store,
                    &parent,
                    depth - cur_depth - 1,
                    |new_info| {
                        pending_infos.push(new_info.clone());
                    },
                )?;

                let (block, _block_info) = store.get_block(&block_info.block_hash)?;
                Ok((pending_infos, block))
            }
        })
        .await
        .map_err(|e| StorageError::BackendError(Box::new(e)))
        .and_then(std::convert::identity)?;

        self.pending_infos.append(&mut pending_infos);

        Ok(block)
    }
}

struct SendState<S, E> {
    sink: Pin<Box<S>>,
    iter: BlockIterState,
    pending: Option<Result<Block, E>>,
}

impl<S, E> SendState<S, E>
where
    S: Sink<Result<Block, E>, Error = E>,
    E: From<StorageError>,
{
    async fn r#continue(&mut self) -> Result<bool, S::Error> {
        let sink = self.sink.as_mut();
        if let Some(item) = self.pending.take() {
            sink.send(item).await?;
        }

        let has_next = self.iter.has_next();

        if !has_next {
            sink.close().await?;
        }

        Ok(has_next)
    }

    async fn fill_sink(&mut self, store: Pool<ConnectionManager>) -> Result<(), S::Error> {
        assert!(self.iter.has_next());
        loop {
            let item = self.iter.get_next(store).await.map_err(Into::into);
            self.sink.as_mut().start_send(item)?
        }
    }
}
