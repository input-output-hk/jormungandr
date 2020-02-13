use crate::{
    blockcfg::{Block, HeaderHash},
    start_up::{NodeStorage, NodeStorageConnection},
};
use async_trait::async_trait;
use bb8::{ManageConnection, Pool, RunError};
use chain_storage_sqlite_old::{for_path_to_nth_ancestor, BlockInfo};
use futures::{Future as Future01, Sink as Sink01, Stream as Stream01};
use futures03::{
    compat::*,
    prelude::*,
    sink::{Sink, SinkExt},
    stream::{self, Stream},
};
use std::{convert::identity, pin::Pin, sync::Arc};
use tokio02::{sync::Mutex, task::spawn_blocking};
use tokio_compat::runtime;

pub use chain_storage_sqlite_old::Error as StorageError;

async fn run_blocking_storage<F, R>(f: F) -> Result<R, StorageError>
where
    F: FnOnce() -> Result<R, StorageError> + Send + 'static,
    R: Send + 'static,
{
    spawn_blocking(f)
        .await
        .map_err(|e| StorageError::BackendError(Box::new(e)))
        .and_then(identity)
}

async fn run_blocking_with_connection<F, R>(
    pool: &Pool<ConnectionManager>,
    f: F,
) -> Result<R, StorageError>
where
    F: FnOnce(&mut NodeStorageConnection) -> Result<R, StorageError> + Send + 'static,
    R: Send + 'static,
{
    pool.run(|mut connection| async move {
        spawn_blocking(move || match f(&mut connection) {
            Ok(r) => Ok((r, connection)),
            Err(r) => Err((r, connection)),
        })
        .await
        .unwrap()
    })
    .await
    .map_err(|e| match e {
        RunError::User(e) => e,
        e => StorageError::BackendError(Box::new(e)),
    })
}

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
        run_blocking_storage(move || inner.connect()).await
    }

    async fn is_valid(&self, conn: Self::Connection) -> Result<Self::Connection, Self::Error> {
        run_blocking_storage(move || conn.ping().and(Ok(conn))).await
    }

    fn has_broken(&self, conn: &mut Self::Connection) -> bool {
        conn.ping().is_ok()
    }
}

#[derive(Clone)]
pub struct Storage03 {
    pool: Pool<ConnectionManager>,

    // All write operations must be performed only via this lock. The lock helps
    // us to ensure that all of the write operations are performed in the right
    // sequence. Otherwise they can be performed out of the expected order (for
    // example, by different tokio executors) which eventually leads to a panic
    // because the block data would be inconsistent at the time of a write.
    write_lock: Arc<Mutex<()>>,
}

// Compatibility layer for using new storage with old futures API.
#[derive(Clone)]
pub struct Storage {
    inner: Storage03,
}

pub struct Ancestor {
    pub header_hash: HeaderHash,
    pub distance: u64,
}

struct BlockIterState {
    to_length: u64,
    cur_length: u64,
    pending_infos: Vec<BlockInfo<HeaderHash>>,
}

impl Storage03 {
    pub fn new(storage: NodeStorage) -> Self {
        let mut rt = runtime::Builder::new()
            .name_prefix("new-storage-worker-")
            .core_threads(1)
            .build()
            .unwrap();

        rt.block_on_std(async move {
            let manager = ConnectionManager::new(storage);
            let pool = Pool::builder().build(manager).await.unwrap();
            let write_lock = Arc::new(Mutex::new(()));

            Storage03 { pool, write_lock }
        })
    }

    async fn run<F, R>(&self, f: F) -> Result<R, StorageError>
    where
        F: FnOnce(&mut NodeStorageConnection) -> Result<R, StorageError> + Send + 'static,
        R: Send + 'static,
    {
        run_blocking_with_connection(&self.pool, f).await
    }

    pub async fn get_tag(&self, tag: String) -> Result<Option<HeaderHash>, StorageError> {
        self.run(move |connection| connection.get_tag(&tag)).await
    }

    pub async fn put_tag(&self, tag: String, header_hash: HeaderHash) -> Result<(), StorageError> {
        let _ = self.write_lock.lock().await;
        self.run(move |connection| connection.put_tag(&tag, &header_hash))
            .await
    }

    pub async fn get(&self, header_hash: HeaderHash) -> Result<Option<Block>, StorageError> {
        self.run(move |connection| match connection.get_block(&header_hash) {
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
        self.run(move |connection| match connection.get_block(&header_hash) {
            Err(StorageError::BlockNotFound) => Ok(None),
            Ok(v) => Ok(Some(v)),
            Err(e) => Err(e),
        })
        .await
    }

    pub async fn block_exists(&self, header_hash: HeaderHash) -> Result<bool, StorageError> {
        self.run(
            move |connection| match connection.block_exists(&header_hash) {
                Err(StorageError::BlockNotFound) => Ok(false),
                Ok(r) => Ok(r),
                Err(e) => Err(e),
            },
        )
        .await
    }

    pub async fn put_block(&self, block: Block) -> Result<(), StorageError> {
        let _ = self.write_lock.lock().await;
        self.run(move |connection| match connection.put_block(&block) {
            Err(StorageError::BlockNotFound) => unreachable!(),
            Err(e) => Err(e),
            Ok(()) => Ok(()),
        })
        .await
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
            .run(move |connection| match connection.is_ancestor(&from, &to) {
                Ok(Some(distance)) => match connection.get_block_info(&to) {
                    Ok(to_info) => Ok(BlockIterState::new(to_info, distance)),
                    Err(e) => Err(e),
                },
                Ok(None) => Err(StorageError::CannotIterate),
                Err(e) => Err(e),
            })
            .await?;

        let pool = self.pool.clone();

        Ok(stream::unfold(
            (init_state, pool),
            |(mut state, pool)| async move {
                if !state.has_next() {
                    return None;
                }
                let res = state.get_next(pool.clone()).await;
                Some((res, (state, pool)))
            },
        ))
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
        S: Sink<Result<Block, E>>,
        E: From<StorageError>,
    {
        let mut sink = sink;

        let res = self
            .run(move |connection| {
                connection.get_block_info(&to).map(|to_info| {
                    let depth = depth.unwrap_or(to_info.chain_length - 1);
                    BlockIterState::new(to_info, depth)
                })
            })
            .await;

        match res {
            Ok(mut iter) => {
                while iter.has_next() {
                    let item = iter.get_next(self.pool.clone()).await.map_err(Into::into);
                    sink.send(item).await?;
                }
                sink.close().await?;
            }
            Err(e) => {
                sink.send_all(&mut stream::once(Box::pin(async { Ok(Err(e.into())) })))
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
        self.run(move |connection| {
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

impl Storage {
    pub fn new(storage: NodeStorage) -> Self {
        Self {
            inner: Storage03::new(storage),
        }
    }

    pub fn get_tag(
        &self,
        tag: String,
    ) -> impl Future01<Item = Option<HeaderHash>, Error = StorageError> {
        let inner = self.inner.clone();
        Compat::new(Box::pin(async move { inner.get_tag(tag).await }))
    }

    pub fn put_tag(
        &self,
        tag: String,
        header_hash: HeaderHash,
    ) -> impl Future01<Item = (), Error = StorageError> {
        let inner = self.inner.clone();
        Compat::new(Box::pin(
            async move { inner.put_tag(tag, header_hash).await },
        ))
    }

    pub fn get(
        &self,
        header_hash: HeaderHash,
    ) -> impl Future01<Item = Option<Block>, Error = StorageError> {
        let inner = self.inner.clone();
        Compat::new(Box::pin(async move { inner.get(header_hash).await }))
    }

    pub fn get_with_info(
        &self,
        header_hash: HeaderHash,
    ) -> impl Future01<Item = Option<(Block, BlockInfo<HeaderHash>)>, Error = StorageError> {
        let inner = self.inner.clone();
        Compat::new(Box::pin(
            async move { inner.get_with_info(header_hash).await },
        ))
    }

    pub fn block_exists(
        &self,
        header_hash: HeaderHash,
    ) -> impl Future01<Item = bool, Error = StorageError> {
        let inner = self.inner.clone();
        Compat::new(Box::pin(
            async move { inner.block_exists(header_hash).await },
        ))
    }

    pub fn put_block(&self, block: Block) -> impl Future01<Item = (), Error = StorageError> {
        let inner = self.inner.clone();
        Compat::new(Box::pin(async move { inner.put_block(block).await }))
    }

    pub fn stream_from_to(
        &self,
        from: HeaderHash,
        to: HeaderHash,
    ) -> impl Future01<Item = impl Stream01<Item = Block, Error = StorageError>, Error = StorageError>
    {
        let inner = self.inner.clone();
        let fut = async move {
            inner
                .stream_from_to(from, to)
                .map_ok(|stream| Compat::new(Box::pin(stream)))
                .await
        };
        let res = Compat::new(Box::pin(fut));
        res
    }

    pub fn send_branch<S, E>(
        &self,
        to: HeaderHash,
        depth: Option<u64>,
        sink: S,
    ) -> impl Future01<Item = (), Error = S::SinkError>
    where
        S: Sink01<SinkItem = Result<Block, E>>,
        E: From<StorageError>,
    {
        let inner = self.inner.clone();
        Compat::new(Box::pin(async move {
            inner
                .send_branch(to, depth, Box::pin(sink.sink_compat()))
                .await
        }))
    }

    pub fn find_closest_ancestor(
        &self,
        checkpoints: Vec<HeaderHash>,
        descendant: HeaderHash,
    ) -> impl Future01<Item = Option<Ancestor>, Error = StorageError> {
        let inner = self.inner.clone();
        Compat::new(Box::pin(async move {
            inner.find_closest_ancestor(checkpoints, descendant).await
        }))
    }
}

impl BlockIterState {
    fn new(to_info: BlockInfo<HeaderHash>, distance: u64) -> Self {
        BlockIterState {
            to_length: to_info.chain_length,
            cur_length: to_info.chain_length - distance,
            pending_infos: vec![to_info],
        }
    }

    fn has_next(&self) -> bool {
        self.cur_length < self.to_length
    }

    async fn get_next(&mut self, pool: Pool<ConnectionManager>) -> Result<Block, StorageError> {
        assert!(self.has_next());

        self.cur_length += 1;

        let block_info = self.pending_infos.pop().unwrap();

        let cur_depth = self.cur_length;

        let (mut pending_infos, block) = run_blocking_with_connection(&pool, move |mut store| {
            if block_info.chain_length == cur_depth {
                // We've seen this block on a previous ancestor traversal.
                let (block, _block_info) = store.get_block(&block_info.block_hash)?;
                Ok((Vec::new(), block))
            } else {
                // We don't have this block yet, so search back from
                // the furthest block that we do have.
                assert!(cur_depth < block_info.chain_length);
                let depth = block_info.chain_length;
                let parent = block_info.parent_id();
                let mut pending_infos = Vec::new();
                pending_infos.push(block_info);
                let block_info = for_path_to_nth_ancestor(
                    &mut store,
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
        .await?;

        self.pending_infos.append(&mut pending_infos);

        Ok(block)
    }
}
