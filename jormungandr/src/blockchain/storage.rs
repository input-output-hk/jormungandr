use crate::{
    blockcfg::{Block, HeaderHash},
    intercom::{self, ReplySendError, ReplyStreamHandle, ReplyStreamHandle03},
    start_up::{NodeStorage, NodeStorageConnection},
};
use chain_storage_sqlite_old::{for_path_to_nth_ancestor, BlockInfo};
use futures::{Future as Future01, Stream as Stream01};
use futures03::{
    channel::{mpsc, oneshot},
    compat::Compat,
    prelude::*,
    ready,
    stream::FusedStream,
};
use pin_utils::{unsafe_pinned, unsafe_unpinned};
use r2d2::{ManageConnection, Pool};
use slog::Logger;
use tokio02::task::spawn_blocking;

use std::convert::identity;
use std::error::Error;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

const BLOCK_STREAM_BUFFER_SIZE: usize = 32;

// How many stream items to leave unaccounted for in PumpedStream
// before priming the pump again.
const PUMP_PRESSURE_MARGIN: usize = 4;

const SYNC_QUERIES_CHANNEL_BOUND: usize = 32;

pub use chain_storage_sqlite_old::Error as StorageError;

async fn run_blocking_with_connection<F, T, E>(pool: Pool<ConnectionManager>, f: F) -> Result<T, E>
where
    F: FnOnce(&mut NodeStorageConnection) -> Result<T, E>,
    F: Send + 'static,
    T: Send + 'static,
    E: Error + From<StorageError> + Send + 'static,
{
    spawn_blocking(move || {
        let mut connection = pool
            .get()
            .map_err(|e| StorageError::BackendError(e.into()))?;
        f(&mut connection)
    })
    .await
    .unwrap()
}

async fn pump_block_sink<T, F>(
    iter: Box<BlockIterState<T, F>>,
    pool: &Pool<ConnectionManager>,
    sink: &mut ReplyStreamHandle03<T>,
) -> Result<BlockIteration<T, F>, ReplySendError>
where
    F: FnMut(Block) -> T,
    F: Send + 'static,
    T: Send + 'static,
{
    let mut sink1 = sink.clone();
    match run_blocking_with_connection(pool.clone(), move |connection| {
        iter.fill_sink(connection, &mut sink1)
            .map_err(StreamingError::Sending)
    })
    .await
    {
        Ok(BlockIteration::Continue(iter)) => {
            future::poll_fn(|cx| sink.poll_ready(cx)).await?;
            Ok(BlockIteration::Continue(iter))
        }
        Ok(BlockIteration::Break) => Ok(BlockIteration::Break),
        Err(StreamingError::Storage(e)) => {
            sink.send(Err(e.into())).await?;
            Ok(BlockIteration::Break)
        }
        Err(StreamingError::Sending(e)) => Err(e),
    }
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

impl ManageConnection for ConnectionManager {
    type Connection = NodeStorageConnection;
    type Error = StorageError;

    fn connect(&self) -> Result<Self::Connection, Self::Error> {
        self.inner.connect()
    }

    fn is_valid(&self, conn: &mut Self::Connection) -> Result<(), Self::Error> {
        conn.ping()
    }

    fn has_broken(&self, _conn: &mut Self::Connection) -> bool {
        false
    }
}

#[derive(Clone)]
pub struct Storage03 {
    pool: Pool<ConnectionManager>,
    sync_queries_sender: mpsc::Sender<Query>,
    logger: Logger,
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

struct BlockIterState<T, F> {
    to_length: u64,
    cur_length: u64,
    transform: F,
    pending_infos: Vec<BlockInfo<HeaderHash>>,
    pending_item: Option<Result<T, intercom::Error>>,
}

enum BlockIteration<T, F> {
    Continue(Box<BlockIterState<T, F>>),
    Break,
}

#[derive(Debug, thiserror::Error)]
enum StreamingError {
    #[error("error accessing storage")]
    Storage(
        #[from]
        #[source]
        StorageError,
    ),
    #[error("failed to send block")]
    Sending(
        #[from]
        #[source]
        ReplySendError,
    ),
}

enum Query {
    PutBlock(Block, oneshot::Sender<Result<(), StorageError>>),
    PutTag(
        String,
        HeaderHash,
        oneshot::Sender<Result<(), StorageError>>,
    ),
}

pub struct StorageSyncQueryExecutor {
    queries: mpsc::Receiver<Query>,
    pool: Pool<ConnectionManager>,
}

impl StorageSyncQueryExecutor {
    pub async fn start(self) {
        let StorageSyncQueryExecutor { mut queries, pool } = self;

        let mut queries = stream::poll_fn(move |cx| {
            let mut queries_vec = Vec::new();
            loop {
                let queries_queue = Pin::new(&mut queries);
                match queries_queue.poll_next(cx) {
                    Poll::Ready(Some(query)) => queries_vec.push(query),
                    Poll::Ready(None) => return Poll::Ready(None),
                    Poll::Pending => {
                        if queries_vec.is_empty() {
                            return Poll::Pending;
                        }
                        return Poll::Ready(Some(queries_vec));
                    }
                }
            }
        });

        while let Some(queries_chunk) = queries.next().await {
            let _ = run_blocking_with_connection(pool.clone(), move |connection| {
                for query in queries_chunk {
                    match query {
                        Query::PutBlock(block, callback) => {
                            let _ = callback.send(match connection.put_block(&block) {
                                Err(StorageError::BlockNotFound) => unreachable!(),
                                Err(e) => Err(e),
                                Ok(()) => Ok(()),
                            });
                        }
                        Query::PutTag(tag, header_hash, callback) => {
                            let _ = callback.send(connection.put_tag(&tag, &header_hash));
                        }
                    }
                }
                Ok::<_, StorageError>(())
            })
            .await;
        }
    }
}

impl Storage03 {
    pub fn new(storage: NodeStorage, logger: Logger) -> (Self, StorageSyncQueryExecutor) {
        let manager = ConnectionManager::new(storage);
        let pool = Pool::builder().build(manager).unwrap();
        let (sync_queries_sender, sync_queries_receiver) =
            mpsc::channel(SYNC_QUERIES_CHANNEL_BOUND);

        let sync_query_executor = StorageSyncQueryExecutor {
            queries: sync_queries_receiver,
            pool: pool.clone(),
        };

        let storage = Storage03 {
            pool,
            sync_queries_sender,
            logger,
        };

        (storage, sync_query_executor)
    }

    async fn run<F, T, E>(&self, f: F) -> Result<T, E>
    where
        F: FnOnce(&mut NodeStorageConnection) -> Result<T, E>,
        F: Send + 'static,
        T: Send + 'static,
        E: Error + From<StorageError> + Send + 'static,
    {
        run_blocking_with_connection(self.pool.clone(), f).await
    }

    pub async fn get_tag(&self, tag: String) -> Result<Option<HeaderHash>, StorageError> {
        self.run(move |connection| connection.get_tag(&tag)).await
    }

    pub async fn put_tag(
        &mut self,
        tag: String,
        header_hash: HeaderHash,
    ) -> Result<(), StorageError> {
        let (tx, rx) = oneshot::channel();
        self.sync_queries_sender
            .send(Query::PutTag(tag, header_hash, tx))
            .await
            .unwrap();
        rx.await.unwrap()
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

    pub async fn put_block(&mut self, block: Block) -> Result<(), StorageError> {
        let (tx, rx) = oneshot::channel();
        self.sync_queries_sender
            .send(Query::PutBlock(block, tx))
            .await
            .unwrap();
        rx.await.unwrap()
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
    ) -> Result<impl Stream<Item = Result<Block, intercom::Error>>, StorageError> {
        let iter = self
            .run(move |connection| match connection.is_ancestor(&from, &to) {
                Ok(Some(distance)) => match connection.get_block_info(&to) {
                    Ok(to_info) => Ok(Box::new(BlockIterState::new(to_info, distance, identity))),
                    Err(e) => Err(e),
                },
                Ok(None) => Err(StorageError::CannotIterate),
                Err(e) => Err(e),
            })
            .await?;

        let (rh, rs) = intercom::stream_reply03(BLOCK_STREAM_BUFFER_SIZE, self.logger.clone());

        struct PumpState<F> {
            iter: Box<BlockIterState<Block, F>>,
            pool: Pool<ConnectionManager>,
            handle: ReplyStreamHandle03<Block>,
        }
        let state = PumpState {
            iter,
            pool: self.pool.clone(),
            handle: rh,
        };
        let pump = stream::unfold(state, |mut state| async move {
            match pump_block_sink(state.iter, &state.pool, &mut state.handle)
                .await
                .unwrap_or_else(|e| panic!("unexpected channel error: {:?}", e))
            {
                BlockIteration::Continue(iter) => {
                    let state = PumpState { iter, ..state };
                    Some(((), state))
                }
                BlockIteration::Break => {
                    state
                        .handle
                        .close()
                        .await
                        .unwrap_or_else(|e| panic!("unexpected channel error: {:?}", e));
                    None
                }
            }
        });
        let stream = PumpedStream::new(rs, pump);
        Ok(stream)
    }

    /// Stream a branch ending at `to` and starting from the ancestor
    /// at `depth` or at the first ancestor since genesis block
    /// if `depth` is given as `None`.
    ///
    /// This function uses buffering in the in-memory channel to reduce
    /// synchronization overhead.
    pub async fn send_branch(
        &self,
        to: HeaderHash,
        depth: Option<u64>,
        sink: ReplyStreamHandle03<Block>,
    ) -> Result<(), ReplySendError> {
        self.send_branch_with(to, depth, sink, identity).await
    }

    /// Like `send_branch`, but with a transformation function applied
    /// to the block content before sending to the in-memory channel.
    pub async fn send_branch_with<T, F>(
        &self,
        to: HeaderHash,
        depth: Option<u64>,
        mut sink: ReplyStreamHandle03<T>,
        transform: F,
    ) -> Result<(), ReplySendError>
    where
        F: FnMut(Block) -> T,
        F: Send + 'static,
        T: Send + 'static,
    {
        let res = self
            .run(move |connection| {
                connection.get_block_info(&to).map(|to_info| {
                    let depth = depth.unwrap_or(to_info.chain_length - 1);
                    Box::new(BlockIterState::new(to_info, depth, transform))
                })
            })
            .await;

        match res {
            Ok(mut iter) => {
                while let BlockIteration::Continue(new_iter_state) =
                    pump_block_sink(iter, &self.pool, &mut sink).await?
                {
                    iter = new_iter_state;
                }
            }
            Err(e) => {
                sink.send(Err(e.into())).await?;
            }
        }

        sink.close().await?;
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
    /// get back to the future
    pub fn back_to_the_future(&self) -> &Storage03 {
        &self.inner
    }

    pub fn new(storage: NodeStorage, logger: Logger) -> (Self, StorageSyncQueryExecutor) {
        let (storage_wrapper, sync_query_executor) = Storage03::new(storage, logger);

        (
            Self {
                inner: storage_wrapper,
            },
            sync_query_executor,
        )
    }

    pub fn get_tag(
        &self,
        tag: String,
    ) -> impl Future01<Item = Option<HeaderHash>, Error = StorageError> {
        let inner = self.inner.clone();
        Compat::new(Box::pin(async move { inner.get_tag(tag).await }))
    }

    pub fn put_tag(
        &mut self,
        tag: String,
        header_hash: HeaderHash,
    ) -> impl Future01<Item = (), Error = StorageError> {
        let mut inner = self.inner.clone();
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

    pub fn put_block(&mut self, block: Block) -> impl Future01<Item = (), Error = StorageError> {
        let mut inner = self.inner.clone();
        Compat::new(Box::pin(async move { inner.put_block(block).await }))
    }

    pub fn stream_from_to(
        &self,
        from: HeaderHash,
        to: HeaderHash,
    ) -> impl Future01<Item = impl Stream01<Item = Block, Error = intercom::Error>, Error = StorageError>
    {
        let inner = self.inner.clone();
        let fut = async move {
            inner
                .stream_from_to(from, to)
                .map_ok(|stream| Box::pin(stream).compat())
                .await
        };
        Box::pin(fut).compat()
    }

    pub fn send_branch(
        &self,
        to: HeaderHash,
        depth: Option<u64>,
        sink: ReplyStreamHandle<Block>,
    ) -> impl Future01<Item = (), Error = ReplySendError> {
        let inner = self.inner.clone();
        let fut = async move { inner.send_branch(to, depth, sink.into_03()).await };
        Box::pin(fut).compat()
    }

    pub fn send_branch_with<T, F>(
        &self,
        to: HeaderHash,
        depth: Option<u64>,
        sink: ReplyStreamHandle<T>,
        transform: F,
    ) -> impl Future01<Item = (), Error = ReplySendError>
    where
        F: FnMut(Block) -> T,
        F: Send + 'static,
        T: Send + 'static,
    {
        let inner = self.inner.clone();
        let fut = async move {
            inner
                .send_branch_with(to, depth, sink.into_03(), transform)
                .await
        };
        Box::pin(fut).compat()
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

struct PumpedStream<S, P> {
    pump: P,
    stream: S,
    pressure: usize,
}

impl<S: Unpin, P: Unpin> Unpin for PumpedStream<S, P> {}

impl<S, P> PumpedStream<S, P> {
    unsafe_pinned!(pump: P);
    unsafe_pinned!(stream: S);
    unsafe_unpinned!(pressure: usize);
}

const PUMP_PRESSURE_FULL: usize = BLOCK_STREAM_BUFFER_SIZE - PUMP_PRESSURE_MARGIN;

impl<S, P> PumpedStream<S, P>
where
    P: Stream<Item = ()>,
{
    fn new(stream: S, pump: P) -> Self {
        PumpedStream {
            pump,
            stream,
            pressure: PUMP_PRESSURE_FULL,
        }
    }
}

impl<S, P> PumpedStream<S, P>
where
    P: Stream<Item = ()> + FusedStream,
{
    fn poll_pump(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<()> {
        if self.pump.is_terminated() {
            return Poll::Pending;
        }
        ready!(self.as_mut().pump().poll_next(cx));
        *self.as_mut().pressure() = PUMP_PRESSURE_FULL;
        ().into()
    }
}

impl<S, P> Stream for PumpedStream<S, P>
where
    S: Stream,
    P: Stream<Item = ()> + FusedStream,
{
    type Item = S::Item;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<S::Item>> {
        loop {
            // Avoid polling on the costly pump machinery while we can expect
            // the stream to produce values.
            if self.pressure == 0 {
                match self.as_mut().poll_pump(cx) {
                    Poll::Pending => {
                        return self.as_mut().stream().poll_next(cx);
                    }
                    Poll::Ready(()) => {}
                }
            } else {
                match self.as_mut().stream().poll_next(cx) {
                    Poll::Ready(Some(item)) => {
                        *self.as_mut().pressure() -= 1;
                        return Some(item).into();
                    }
                    Poll::Ready(None) => return None.into(),
                    Poll::Pending => {
                        ready!(self.as_mut().poll_pump(cx));
                    }
                }
            }
        }
    }
}

impl<T, F> BlockIterState<T, F>
where
    F: FnMut(Block) -> T,
{
    fn new(to_info: BlockInfo<HeaderHash>, distance: u64, transform: F) -> Self {
        BlockIterState {
            to_length: to_info.chain_length,
            cur_length: to_info.chain_length - distance,
            transform,
            pending_infos: vec![to_info],
            pending_item: None,
        }
    }

    fn has_next(&self) -> bool {
        self.cur_length < self.to_length
    }

    // Iterates the blocks accordingly to this iterator's properties
    // and sends them to the intercom channel until
    // the iteration is complete, the channel is full, or an error occurs.
    // If a storage error is encountered, it is also sent to the channel,
    // after which iteration terminates.
    fn fill_sink(
        mut self: Box<Self>,
        store: &mut NodeStorageConnection,
        sink: &mut ReplyStreamHandle03<T>,
    ) -> Result<BlockIteration<T, F>, ReplySendError> {
        if let Some(item) = self.pending_item.take() {
            let is_err = item.is_err();
            if !self.try_send_item(item, sink)? {
                return Ok(BlockIteration::Continue(self));
            } else if is_err {
                return Ok(BlockIteration::Break);
            }
        }
        while self.has_next() {
            match self.get_next_block(store) {
                Ok(block) => {
                    let content = (self.transform)(block);
                    if !self.try_send_item(Ok(content), sink)? {
                        return Ok(BlockIteration::Continue(self));
                    }
                }
                Err(e) => {
                    if self.try_send_item(Err(e.into()), sink)? {
                        return Ok(BlockIteration::Break);
                    } else {
                        return Ok(BlockIteration::Continue(self));
                    }
                }
            }
        }
        Ok(BlockIteration::Break)
    }

    fn get_next_block(&mut self, store: &mut NodeStorageConnection) -> Result<Block, StorageError> {
        debug_assert!(self.has_next());
        self.cur_length += 1;

        let block_info = self.pending_infos.pop().unwrap();
        let cur_length = self.cur_length;

        if block_info.chain_length == cur_length {
            // We've seen this block on a previous ancestor traversal.
            let (block, _block_info) = store.get_block(&block_info.block_hash)?;
            Ok(block)
        } else {
            // We don't have this block yet, so search back from
            // the furthest block that we do have.
            assert!(cur_length < block_info.chain_length);
            let length = block_info.chain_length;
            let parent = block_info.parent_id();
            let mut pending_infos = Vec::new();
            pending_infos.push(block_info);
            let block_info =
                for_path_to_nth_ancestor(store, &parent, length - cur_length - 1, |new_info| {
                    pending_infos.push(new_info.clone());
                })?;

            let (block, _block_info) = store.get_block(&block_info.block_hash)?;
            self.pending_infos.append(&mut pending_infos);
            Ok(block)
        }
    }

    fn try_send_item(
        &mut self,
        item: Result<T, intercom::Error>,
        sink: &mut ReplyStreamHandle03<T>,
    ) -> Result<bool, ReplySendError> {
        sink.try_send_item(item).map(|()| true).or_else(|e| {
            if e.is_full() {
                self.pending_item = Some(e.into_inner());
                Ok(false)
            } else {
                Err(e.into_send_error())
            }
        })
    }
}
