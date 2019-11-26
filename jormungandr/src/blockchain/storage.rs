use crate::{
    blockcfg::{Block, HeaderHash},
    start_up::NodeStorage,
};
use chain_storage::store::{for_path_to_nth_ancestor, BlockInfo, BlockStore};
use tokio::prelude::future::Either;
use tokio::prelude::*;
use tokio::sync::lock::{Lock, LockGuard};

pub use chain_storage::error::Error as StorageError;

#[derive(Clone)]
pub struct Storage {
    inner: Lock<NodeStorage>,
}

pub struct BlockStream {
    lock: Lock<NodeStorage>,
    state: BlockIterState,
}

struct BlockIterState {
    to_depth: u64,
    cur_depth: u64,
    pending_infos: Vec<BlockInfo<HeaderHash>>,
}

impl Storage {
    pub fn new(storage: NodeStorage) -> Self {
        Storage {
            inner: Lock::new(storage),
        }
    }

    #[deprecated(since = "new blockchain API", note = "use the stream iterator instead")]
    pub fn get_inner(&self) -> impl Future<Item = LockGuard<NodeStorage>, Error = StorageError> {
        let mut inner = self.inner.clone();

        future::poll_fn(move || Ok(inner.poll_lock()))
    }

    pub fn get_tag(
        &self,
        tag: String,
    ) -> impl Future<Item = Option<HeaderHash>, Error = StorageError> {
        let mut inner = self.inner.clone();

        future::poll_fn(move || Ok(inner.poll_lock())).and_then(move |guard| {
            match guard.get_tag(&tag) {
                Err(error) => future::err(error),
                Ok(res) => future::ok(res),
            }
        })
    }

    pub fn put_tag(
        &mut self,
        tag: String,
        header_hash: HeaderHash,
    ) -> impl Future<Item = (), Error = StorageError> {
        let mut inner = self.inner.clone();

        future::poll_fn(move || Ok(inner.poll_lock())).and_then(move |mut guard| {
            match guard.put_tag(&tag, &header_hash) {
                Err(error) => future::err(error),
                Ok(res) => future::ok(res),
            }
        })
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

    pub fn get_with_info(
        &self,
        header_hash: HeaderHash,
    ) -> impl Future<Item = Option<(Block, BlockInfo<HeaderHash>)>, Error = StorageError> {
        let mut inner = self.inner.clone();

        future::poll_fn(move || Ok(inner.poll_lock())).and_then(move |guard| {
            match guard.get_block(&header_hash) {
                Err(StorageError::BlockNotFound) => future::ok(None),
                Err(error) => future::err(error),
                Ok(v) => future::ok(Some(v)),
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

    /// Return values:
    /// - `Ok(stream)` - `from` is ancestor of `to`, returns blocks between them
    /// - `Err(CannotIterate)` - `from` is not ancestor of `to`
    /// - `Err(BlockNotFound)` - `from` or `to` was not found
    /// - `Err(_)` - some other storage error
    pub fn stream_from_to(
        &self,
        from: HeaderHash,
        to: HeaderHash,
    ) -> impl Future<Item = BlockStream, Error = StorageError> {
        let mut inner = self.inner.clone();
        let inner_2 = self.inner.clone();

        future::poll_fn(move || Ok(inner.poll_lock())).and_then(move |store| {
            match store.is_ancestor(&from, &to) {
                Err(error) => future::err(error),
                Ok(None) => future::err(StorageError::CannotIterate),
                Ok(Some(distance)) => match store.get_block_info(&to) {
                    Err(error) => future::err(error),
                    Ok(to_info) => future::ok(BlockStream {
                        lock: inner_2,
                        state: BlockIterState::new(to_info, distance),
                    }),
                },
            }
        })
    }

    /// Like `stream_from_to` with the same error meanings, but using buffering
    /// in the sink to reduce lock contention.
    pub fn send_from_to<S, E>(
        &self,
        from: HeaderHash,
        to: HeaderHash,
        sink: S,
    ) -> impl Future<Item = (), Error = S::SinkError>
    where
        S: Sink<SinkItem = Result<Block, E>>,
        E: From<StorageError>,
    {
        let mut inner = self.inner.clone();
        let mut inner_2 = self.inner.clone();

        future::poll_fn(move || Ok(inner.poll_lock()))
            .and_then(move |store| {
                store
                    .is_ancestor(&from, &to)
                    .and_then(|ancestry| match ancestry {
                        None => Err(StorageError::CannotIterate),
                        Some(distance) => store
                            .get_block_info(&to)
                            .map(|to_info| BlockIterState::new(to_info, distance)),
                    })
            })
            .then(move |res| match res {
                Ok(iter) => {
                    let mut state = SendState {
                        sink,
                        iter,
                        pending: None,
                    };
                    let fut = future::poll_fn(move || {
                        while try_ready!(state.poll_continue()) {
                            let mut store = try_ready!(Ok(inner_2.poll_lock()));
                            try_ready!(state.fill_sink(&mut store));
                        }
                        Ok(().into())
                    });
                    Either::A(fut)
                }
                Err(e) => {
                    let fut = sink
                        .send_all(stream::once(Ok(Err(e.into()))))
                        .map(|(_, _)| ());
                    Either::B(fut)
                }
            })
    }

    pub fn get_checkpoints(
        &self,
        tip: HeaderHash,
    ) -> impl Future<Item = Vec<HeaderHash>, Error = StorageError> {
        let mut inner = self.inner.clone();
        future::poll_fn(move || Ok(inner.poll_lock())).and_then(move |store| {
            let tip_info = store.get_block_info(&tip)?;
            let mut checkpoints = Vec::new();
            assert!(tip_info.depth > 0);
            for_path_to_nth_ancestor(&*store, &tip, tip_info.depth - 1, |block_info| {
                checkpoints.push(block_info.block_hash.clone());
            })?;
            Ok(checkpoints)
        })
    }

    pub fn find_closest_ancestor(
        &self,
        checkpoints: Vec<HeaderHash>,
        descendant: HeaderHash,
    ) -> impl Future<Item = Option<HeaderHash>, Error = StorageError> {
        let mut inner = self.inner.clone();
        future::poll_fn(move || Ok(inner.poll_lock())).and_then(move |store| {
            let mut ancestor = None;
            let mut closest_found = std::u64::MAX;
            for checkpoint in checkpoints {
                // Checkpoints sent by a peer may not
                // be present locally, so we need to ignore certain errors
                match store.is_ancestor(&checkpoint, &descendant) {
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
            // Could return the distance alongside in a struct?
            Ok(ancestor)
        })
    }
}

impl Stream for BlockStream {
    type Item = Block;
    type Error = StorageError;

    fn poll(&mut self) -> Poll<Option<Block>, Self::Error> {
        if !self.state.has_next() {
            return Ok(Async::Ready(None));
        }

        let mut store = try_ready!(Ok(self.lock.poll_lock()));

        self.state
            .get_next(&mut store)
            .map(|block| Async::Ready(Some(block)))
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

    fn get_next(&mut self, store: &mut NodeStorage) -> Result<Block, StorageError> {
        assert!(self.has_next());

        self.cur_depth += 1;

        let block_info = self.pending_infos.pop().unwrap();

        if block_info.depth == self.cur_depth {
            // We've seen this block on a previous ancestor traversal.
            let (block, _block_info) = store.get_block(&block_info.block_hash)?;
            Ok(block)
        } else {
            // We don't have this block yet, so search back from
            // the furthest block that we do have.
            assert!(self.cur_depth < block_info.depth);
            let depth = block_info.depth;
            let parent = block_info.parent_id();
            self.pending_infos.push(block_info);
            let block_info = for_path_to_nth_ancestor(
                &*store,
                &parent,
                depth - self.cur_depth - 1,
                |new_info| {
                    self.pending_infos.push(new_info.clone());
                },
            )?;

            let (block, _block_info) = store.get_block(&block_info.block_hash)?;
            Ok(block)
        }
    }
}

struct SendState<S, E> {
    sink: S,
    iter: BlockIterState,
    pending: Option<Result<Block, E>>,
}

impl<S, E> SendState<S, E>
where
    S: Sink<SinkItem = Result<Block, E>>,
    E: From<StorageError>,
{
    fn poll_continue(&mut self) -> Poll<bool, S::SinkError> {
        if let Some(item) = self.pending.take() {
            match self.sink.start_send(item)? {
                AsyncSink::Ready => {}
                AsyncSink::NotReady(item) => {
                    self.pending = Some(item);
                    return Ok(Async::NotReady);
                }
            }
        }

        let has_next = self.iter.has_next();

        if has_next {
            // Flush the sink before locking to send more blocks
            try_ready!(self.sink.poll_complete());
        } else {
            try_ready!(self.sink.close());
        }

        Ok(has_next.into())
    }

    fn fill_sink(&mut self, store: &mut NodeStorage) -> Poll<(), S::SinkError> {
        assert!(self.iter.has_next());
        loop {
            let item = self.iter.get_next(store).map_err(Into::into);
            match self.sink.start_send(item)? {
                AsyncSink::Ready => {
                    if !self.iter.has_next() {
                        return Ok(().into());
                    }
                }
                AsyncSink::NotReady(item) => {
                    self.pending = Some(item);
                    return Ok(Async::NotReady);
                }
            }
        }
    }
}
