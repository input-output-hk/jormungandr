use crate::{
    blockcfg::{Block, HeaderHash},
    intercom::{self, ReplySendError, ReplyStreamHandle, ReplyStreamSink},
    start_up::{NodeStorage, NodeStorageConnection},
};
use chain_storage::{for_path_to_nth_ancestor, BlockInfo, Error as StorageError};
use futures::{prelude::*, ready, stream::FusedStream};
use pin_utils::{unsafe_pinned, unsafe_unpinned};
use r2d2::{ManageConnection, Pool};
use slog::Logger;
use thiserror::Error;
use tokio::task;

use std::convert::identity;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

const BLOCK_STREAM_BUFFER_SIZE: usize = 32;

// How many stream items to leave unaccounted for in PumpedStream
// before priming the pump again.
const PUMP_PRESSURE_MARGIN: usize = 4;

#[derive(Debug, Error)]
pub enum Error {
    #[error("block not found")]
    BlockNotFound,
    // FIXME: add BlockId
    #[error("database backend error")]
    BackendError(#[source] StorageError),
    #[error("Block already present in DB")]
    BlockAlreadyPresent,
    #[error("the parent block is missing for the required write")]
    MissingParent,
    #[error("failed to connect to the database")]
    ConnectionFailed(#[source] r2d2::Error),
    #[error("cannot iterate between the 2 given blocks")]
    CannotIterate,
}

impl From<StorageError> for Error {
    fn from(source: StorageError) -> Self {
        match source {
            StorageError::BlockNotFound => Error::BlockNotFound,
            StorageError::BlockAlreadyPresent => Error::BlockAlreadyPresent,
            StorageError::MissingParent => Error::MissingParent,
            e => Error::BackendError(e),
        }
    }
}

async fn run_blocking_with_connection<F, T, E>(pool: Pool<ConnectionManager>, f: F) -> Result<T, E>
where
    F: FnOnce(&mut NodeStorageConnection) -> Result<T, E>,
    F: Send + 'static,
    T: Send + 'static,
    E: From<Error> + Send + 'static,
{
    task::spawn_blocking(move || {
        let mut connection = pool.get().map_err(Error::ConnectionFailed)?;
        f(&mut connection)
    })
    .await
    .unwrap()
}

struct BlockSinkPump<T, F> {
    iter: Box<BlockIterState<T, F>>,
    pool: Pool<ConnectionManager>,
    sink: ReplyStreamSink<T>,
}

impl<T, F> BlockSinkPump<T, F>
where
    F: FnMut(Block) -> T,
    F: Send + 'static,
    T: Send + 'static,
{
    fn start(
        iter: BlockIterState<T, F>,
        pool: Pool<ConnectionManager>,
        handle: ReplyStreamHandle<T>,
    ) -> Self {
        BlockSinkPump {
            iter: Box::new(iter),
            pool,
            sink: handle.start_sending(),
        }
    }

    async fn pump(mut self) -> Result<Option<Self>, ReplySendError> {
        let mut sink = self.sink.clone();
        let iter = self.iter;
        match run_blocking_with_connection(self.pool.clone(), move |connection| {
            iter.fill_sink(connection, &mut sink)
                .map_err(StreamingError::Sending)
        })
        .await
        {
            Ok(BlockIteration::Continue(iter)) => {
                self.iter = iter;
                let sink = &mut self.sink;
                future::poll_fn(|cx| sink.poll_ready(cx)).await?;
                Ok(Some(self))
            }
            Ok(BlockIteration::Break) => {
                self.sink.close().await?;
                Ok(None)
            }
            Err(StreamingError::Storage(e)) => {
                self.sink.send(Err(e.into())).await?;
                Ok(None)
            }
            Err(StreamingError::Sending(e)) => Err(e),
        }
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
pub struct Storage {
    pool: Pool<ConnectionManager>,
    logger: Logger,
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
    Storage(#[from] Error),
    #[error("failed to send block")]
    Sending(#[from] ReplySendError),
}

impl Storage {
    pub fn new(storage: NodeStorage, logger: Logger) -> Self {
        let manager = ConnectionManager::new(storage);
        let pool = Pool::builder().build(manager).unwrap();

        Storage { pool, logger }
    }

    async fn run<F, T, E>(&self, f: F) -> Result<T, Error>
    where
        F: FnOnce(&mut NodeStorageConnection) -> Result<T, E>,
        F: Send + 'static,
        T: Send + 'static,
        E: Into<Error> + Send + 'static,
    {
        run_blocking_with_connection(self.pool.clone(), |conn| f(conn).map_err(Into::into)).await
    }

    pub async fn get_tag(&self, tag: String) -> Result<Option<HeaderHash>, Error> {
        self.run(move |connection| connection.get_tag(&tag)).await
    }

    pub async fn put_tag(&self, tag: String, header_hash: HeaderHash) -> Result<(), Error> {
        self.run(move |connection| connection.put_tag(&tag, &header_hash))
            .await
    }

    pub async fn get(&self, header_hash: HeaderHash) -> Result<Option<Block>, Error> {
        self.run(move |connection| match connection.get_block(&header_hash) {
            Err(StorageError::BlockNotFound) => Ok(None),
            Ok((block, _block_info)) => Ok(Some(block)),
            Err(e) => Err(e),
        })
        .await
    }

    pub async fn block_exists(&self, header_hash: HeaderHash) -> Result<bool, Error> {
        self.run(
            move |connection| match connection.block_exists(&header_hash) {
                Err(StorageError::BlockNotFound) => Ok(false),
                Ok(r) => Ok(r),
                Err(e) => Err(e),
            },
        )
        .await
    }

    pub async fn get_blocks_by_chain_length(&self, chain_length: u64) -> Result<Vec<Block>, Error> {
        self.run(
            move |connection| match connection.get_blocks_by_chain_length(chain_length) {
                Err(StorageError::BlockNotFound) => Ok(Vec::new()),
                Ok(r) => Ok(r.into_iter().map(|(block, _)| block).collect()),
                Err(e) => Err(e),
            },
        )
        .await
    }

    pub async fn put_block(&self, block: Block) -> Result<(), Error> {
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
    ) -> Result<impl Stream<Item = Result<Block, intercom::Error>>, Error> {
        let (rh, rf) = intercom::stream_reply(BLOCK_STREAM_BUFFER_SIZE, self.logger.clone());
        let iter = self
            .run(
                move |connection| match connection.is_ancestor(&from, &to)? {
                    Some(distance) => {
                        let to_info = connection.get_block_info(&to)?;
                        Ok(BlockIterState::new(to_info, distance, identity))
                    }
                    None => Err(Error::CannotIterate),
                },
            )
            .await?;
        let pump = BlockSinkPump::start(iter, self.pool.clone(), rh);
        let stream = rf.await.expect("unexpected channel error");
        Ok(PumpedStream::new(
            stream,
            stream::unfold(pump, |pump| {
                pump.pump().map(|res| {
                    res.expect("unexpected channel error")
                        .map(|pump| ((), pump))
                })
            }),
        ))
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
        handle: ReplyStreamHandle<Block>,
    ) -> Result<(), ReplySendError> {
        self.send_branch_with(to, depth, handle, identity).await
    }

    /// Like `send_branch`, but with a transformation function applied
    /// to the block content before sending to the in-memory channel.
    pub async fn send_branch_with<T, F>(
        &self,
        to: HeaderHash,
        depth: Option<u64>,
        handle: ReplyStreamHandle<T>,
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
                    BlockIterState::new(to_info, depth, transform)
                })
            })
            .await;

        match res {
            Ok(iter) => {
                let mut pump = BlockSinkPump::start(iter, self.pool.clone(), handle);
                while let Some(new_state) = pump.pump().await? {
                    pump = new_state;
                }
            }
            Err(e) => {
                handle.reply_error(e.into());
            }
        }
        Ok(())
    }

    pub async fn find_closest_ancestor(
        &self,
        checkpoints: Vec<HeaderHash>,
        descendant: HeaderHash,
    ) -> Result<Option<Ancestor>, Error> {
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
        Poll::Ready(())
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
        sink: &mut ReplyStreamSink<T>,
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

    fn get_next_block(&mut self, store: &mut NodeStorageConnection) -> Result<Block, Error> {
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
        sink: &mut ReplyStreamSink<T>,
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
