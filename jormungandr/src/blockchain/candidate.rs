use super::{
    chain::{self, HeaderChainVerifyError},
    chunk_sizes,
    storage::{Storage, StorageError},
};
use crate::blockcfg::{Block, Header, HeaderHash};
use crate::utils::async_msg::MessageQueue;
use chain_core::property::HasHeader;

use futures::future::{self, Either, Loop};
use futures::prelude::*;
use slog::Logger;
use tokio::sync::lock::Lock;
use tokio::timer::{self, delay_queue, DelayQueue};

use std::collections::HashMap;
use std::convert::Infallible;
use std::hint::unreachable_unchecked;
use std::time::Duration;

// derive
use thiserror::Error;

type HeaderStream = MessageQueue<Header>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("blockchain storage error")]
    Storage(
        #[from]
        #[source]
        StorageError,
    ),
    #[error("the incoming header stream is empty")]
    EmptyHeaderStream,
    #[error("the parent block {0} of the first received block header is not found in storage")]
    MissingParentBlock(HeaderHash),
    #[error("the parent hash field {0} of a received block header does not match the hash of the preceding header")]
    BrokenHeaderChain(HeaderHash),
    #[error("block headers do not form a valid chain: {0}")]
    HeaderChainVerificationFailed(
        #[from]
        #[source]
        HeaderChainVerifyError,
    ),
    #[error("unexpected header stream failure")]
    Unexpected,
}

#[derive(Clone)]
pub struct CandidateForest {
    inner: Lock<CandidateForestThickets>,
    storage: Storage,
    logger: Logger,
}

struct CandidateForestThickets {
    candidate_map: HashMap<HeaderHash, Candidate>,
    roots: HashMap<HeaderHash, RootData>,
    root_ttl: Duration,
    expirations: DelayQueue<HeaderHash>,
}

struct Candidate {
    data: CandidateData,
    children: Vec<HeaderHash>,
}

enum CandidateData {
    Header(Header),
    Block(Block),
}

impl Candidate {
    fn from_header(header: Header) -> Self {
        Candidate {
            data: CandidateData::Header(header),
            children: Vec::new(),
        }
    }

    fn from_block(block: Block) -> Self {
        Candidate {
            data: CandidateData::Block(block),
            children: Vec::new(),
        }
    }

    fn has_only_header(&self) -> bool {
        use self::CandidateData::*;
        match self.data {
            Header(_) => true,
            Block(_) => false,
        }
    }

    // FIXME: fix the clone happiness, comes from chain_core::property::HasHeader
    fn header(&self) -> Header {
        use self::CandidateData::*;
        match &self.data {
            Header(header) => header.clone(),
            Block(block) => block.header(),
        }
    }
}

struct RootData {
    expiration_key: delay_queue::Key,
}

mod chain_landing {
    use super::*;

    pub struct State<S> {
        storage: Storage,
        header: Header,
        stream: S,
    }

    impl<S> State<S>
    where
        S: Stream<Item = Header, Error = Error>,
    {
        /// Read the first header from the stream and check that its parent
        /// exists in storage.
        /// Return a future that resolves to a state object.
        /// This method starts the sequence of processing a header chain.
        pub fn start(stream: S, storage: Storage) -> impl Future<Item = Self, Error = Error> {
            stream
                .into_future()
                .map_err(|(err, _)| err)
                .and_then(move |(maybe_first, stream)| match maybe_first {
                    Some(header) => {
                        let parent_hash = header.block_parent_hash();
                        let check_parent_exists = storage.block_exists(parent_hash);
                        let state = State {
                            storage,
                            header,
                            stream,
                        };
                        Ok((check_parent_exists, state))
                    }
                    None => Err(Error::EmptyHeaderStream),
                })
                .and_then(move |(check_parent_exists, state)| {
                    check_parent_exists
                        .map_err(|e| e.into())
                        .and_then(|exists| {
                            if exists {
                                Ok(state)
                            } else {
                                Err(Error::MissingParentBlock(state.header.block_parent_hash())
                                    .into())
                            }
                        })
                })
        }

        /// Read the stream and skip blocks that are already present in the storage.
        /// The end state has the header of the first block that is not present,
        /// but its parent is in storage. The chain also is pre-verified for sanity.
        pub fn skip_present_blocks(self) -> impl Future<Item = Self, Error = Error> {
            future::loop_fn(self, move |state| {
                state
                    .storage
                    .block_exists(state.header.hash())
                    .map_err(|e| e.into())
                    .and_then(move |exists| {
                        if !exists {
                            Either::A(future::ok(Loop::Break(state)))
                        } else {
                            let mut state = Some(state);
                            let read_next = future::poll_fn(move || {
                                let polled = try_ready!(state.as_mut().unwrap().stream.poll());
                                let state = state.take().unwrap();
                                match polled {
                                    Some(header) => {
                                        let parent_hash = header.block_parent_hash();
                                        if parent_hash != state.header.hash() {
                                            return Err(Error::BrokenHeaderChain(parent_hash));
                                        }
                                        chain::pre_verify_link(&header, &state.header)?;
                                        Ok(Loop::Continue(State { header, ..state }).into())
                                    }
                                    None => Ok(Loop::Break(state).into()),
                                }
                            });
                            Either::B(read_next)
                        }
                    })
            })
        }

        pub fn end(self) -> (Header, S) {
            (self.header, self.stream)
        }
    }
}

struct ChainAdvance {
    stream: HeaderStream,
    parent_hash: HeaderHash,
    header: Option<Header>,
    new_hashes: Vec<HeaderHash>,
    logger: Logger,
}

mod chain_advance {
    pub enum Outcome {
        Incomplete,
        Complete,
    }
}

impl ChainAdvance {
    fn try_process_header(
        &mut self,
        header: Header,
        forest: &mut Lock<CandidateForestThickets>,
    ) -> Poll<(), Error> {
        match forest.poll_lock() {
            Async::NotReady => {
                assert!(self.header.is_none());
                self.header = Some(header);
                Ok(Async::NotReady)
            }
            Async::Ready(mut forest) => {
                // If we already have this header as candidate,
                // skip to the next, otherwise validate
                // and store as candidate and a child of its parent.
                let block_hash = header.hash();
                if forest.candidate_map.contains_key(&block_hash) {
                    // Hey, it has the same crypto hash, so it's the
                    // same header, what could possibly go wrong?
                    debug!(
                        self.logger,
                        "block is already cached as a candidate";
                        "hash" => %block_hash,
                    );
                } else {
                    let parent_hash = header.block_parent_hash();
                    if parent_hash != self.parent_hash {
                        return Err(Error::BrokenHeaderChain(parent_hash));
                    }
                    let parent_candidate = forest
                        .candidate_map
                        .get_mut(&parent_hash)
                        .expect("parent candidate should be in the map");
                    chain::pre_verify_link(&header, &parent_candidate.header())?;
                    debug_assert!(!parent_candidate.children.contains(&block_hash));
                    parent_candidate.children.push(block_hash);
                    forest
                        .candidate_map
                        .insert(block_hash, Candidate::from_header(header));
                    debug!(self.logger, "will fetch block"; "hash" => %block_hash);
                    self.new_hashes.push(block_hash);
                }
                self.parent_hash = block_hash;
                Ok(().into())
            }
        }
    }

    fn poll_done(
        &mut self,
        forest: &mut Lock<CandidateForestThickets>,
    ) -> Poll<chain_advance::Outcome, Error> {
        use self::chain_advance::Outcome;

        loop {
            if let Some(header) = self.header.take() {
                try_ready!(self.try_process_header(header, forest));
            } else {
                match try_ready!(self.stream.poll().map_err(|()| Error::Unexpected)) {
                    Some(header) => {
                        try_ready!(self.try_process_header(header, forest));
                    }
                    None => return Ok(Outcome::Complete.into()),
                }
            }
            // TODO: bail out when block data are needed due to new epoch.
            if self.new_hashes.len() >= chunk_sizes::BLOCKS {
                return Ok(Outcome::Incomplete.into());
            }
        }
    }
}

impl CandidateForest {
    pub fn new(storage: Storage, root_ttl: Duration, logger: Logger) -> Self {
        let inner = CandidateForestThickets {
            candidate_map: HashMap::new(),
            roots: HashMap::new(),
            expirations: DelayQueue::new(),
            root_ttl,
        };
        CandidateForest {
            inner: Lock::new(inner),
            storage: storage.clone(),
            logger,
        }
    }

    fn land_header_chain(
        &self,
        stream: HeaderStream,
    ) -> impl Future<Item = ChainAdvance, Error = Error> {
        let storage = self.storage.clone();
        let mut inner = self.inner.clone();
        let logger = self.logger.clone();

        chain_landing::State::start(stream.map_err(|()| unreachable!()), storage)
            .and_then(move |state| state.skip_present_blocks())
            .and_then(move |state| {
                let (header, stream) = state.end();
                // We have got a header that is not in storage, but its
                // parent is.
                // Find an existing root or create a new one.
                future::poll_fn(move || Ok(inner.poll_lock())).and_then(move |mut forest| {
                    let root_parent_hash = header.block_parent_hash();
                    let (root_hash, is_new) = forest.add_or_refresh_root(header);
                    debug!(
                        logger,
                        "landed the header chain, {}",
                        if is_new { "new root" } else { "existing root" };
                        "hash" => %root_hash,
                        "parent" => %root_parent_hash,
                    );
                    let new_hashes = if is_new { vec![root_hash] } else { Vec::new() };
                    let landing = ChainAdvance {
                        stream: stream.into_inner(),
                        parent_hash: root_hash,
                        header: None,
                        new_hashes,
                        logger,
                    };
                    Ok(landing)
                })
            })
    }

    /// Consumes headers from the stream, validating and caching them as
    /// candidate entries with possibly a new root. Returns a future that
    /// resolves to a batch of block hashes to request from the network
    /// and the stream if the process terminated early due to reaching
    /// a limit on the number of blocks or (TODO: implement) needing
    /// block data to validate more blocks with newer leadership information.
    pub fn advance_branch(
        &self,
        header_stream: HeaderStream,
    ) -> impl Future<Item = (Vec<HeaderHash>, Option<HeaderStream>), Error = Error> {
        let mut inner = self.inner.clone();
        self.land_header_chain(header_stream)
            .and_then(move |advance| {
                let mut advance = Some(advance);
                future::poll_fn(move || {
                    use self::chain_advance::Outcome;
                    let done = try_ready!(advance.as_mut().unwrap().poll_done(&mut inner));
                    let advance = advance.take().unwrap();
                    let ret_stream = match done {
                        Outcome::Complete => None,
                        Outcome::Incomplete => Some(advance.stream),
                    };
                    Ok((advance.new_hashes, ret_stream).into())
                })
            })
    }

    /// Puts a block into the cache for later application.
    ///
    /// The block's header must have been earlier registered in a header chain
    /// passed to the `advance_branch` method. If the block is already
    /// in the cache, the block value is not updated and the returned future
    /// resolves successfully.
    pub fn cache_block(&self, block: Block) -> impl Future<Item = (), Error = chain::Error> {
        let header = block.header();
        let block_hash = header.hash();
        let mut inner = self.inner.clone();
        future::poll_fn(move || Ok(inner.poll_lock())).and_then(move |mut forest| {
            use std::collections::hash_map::Entry::*;

            match forest.candidate_map.entry(block_hash) {
                Occupied(mut entry) => match &entry.get().data {
                    CandidateData::Header(header) => {
                        debug_assert!(header.hash() == block_hash);
                        entry.insert(Candidate::from_block(block));
                        Ok(())
                    }
                    CandidateData::Block(block) => {
                        debug_assert!(block.header().hash() == block_hash);
                        Ok(())
                    }
                },
                Vacant(_) => Err(chain::ErrorKind::BlockNotRequested(block_hash).into()),
            }
        })
    }

    pub fn on_applied_block(
        &self,
        block_hash: HeaderHash,
    ) -> impl Future<Item = Vec<Block>, Error = Infallible> {
        let mut inner = self.inner.clone();
        future::poll_fn(move || Ok(inner.poll_lock()))
            .and_then(move |mut forest| Ok(forest.on_applied_block(block_hash)))
    }

    pub fn purge(&self) -> impl Future<Item = (), Error = timer::Error> {
        let mut inner = self.inner.clone();

        // FIXME: this is expected to be called periodically, as it ignores
        // polling deadlines set by the DelayQueue. A rework will be
        // needed to gather all GC activities from here and other blockchain
        // entities to be managed by a common DelayQueue in a separate task,
        // with channels from the garbage-producing tasks to manage
        // expiration.
        future::poll_fn(move || Ok(inner.poll_lock()))
            .and_then(|mut forest| future::poll_fn(move || forest.poll_purge()))
    }
}

impl CandidateForestThickets {
    fn enroll_root(&mut self, root_hash: HeaderHash) -> RootData {
        let expiration_key = self.expirations.insert(root_hash, self.root_ttl);
        RootData { expiration_key }
    }

    fn add_or_refresh_root(&mut self, header: Header) -> (HeaderHash, bool) {
        use std::collections::hash_map::Entry::*;

        let root_hash = header.hash();
        let is_new = match self.roots.entry(root_hash) {
            Vacant(entry) => {
                let expiration_key = self.expirations.insert(root_hash, self.root_ttl);
                entry.insert(RootData { expiration_key });
                let _old = self
                    .candidate_map
                    .insert(root_hash, Candidate::from_header(header));
                debug_assert!(_old.is_none());
                true
            }
            Occupied(entry) => {
                debug_assert!(
                    self.candidate_map
                        .get(&root_hash)
                        .expect("chain pull root candidate should be in the map")
                        .has_only_header(),
                    "a chain pull root candidate should not cache a block",
                );
                self.expirations
                    .reset(&entry.get().expiration_key, self.root_ttl);
                false
            }
        };
        (root_hash, is_new)
    }

    fn remove_root(&mut self, root_hash: &HeaderHash) -> bool {
        match self.roots.remove(&root_hash) {
            Some(root_data) => {
                self.expirations.remove(&root_data.expiration_key);
                true
            }
            None => {
                assert!(!self.candidate_map.contains_key(&root_hash));
                false
            }
        }
    }

    fn on_applied_block(&mut self, block_hash: HeaderHash) -> Vec<Block> {
        use std::collections::hash_map::Entry::*;

        let mut block_avalanche = Vec::new();
        if self.remove_root(&block_hash) {
            let candidate = self
                .candidate_map
                .remove(&block_hash)
                .expect("referential integrity failure in CandidateForest");
            debug_assert!(
                candidate.has_only_header(),
                "a chain pull root candidate should not cache a block",
            );
            let mut child_hashes = candidate.children;
            while let Some(child_hash) = child_hashes.pop() {
                match self.candidate_map.entry(child_hash) {
                    Occupied(entry) => match &entry.get().data {
                        CandidateData::Header(_header) => {
                            debug_assert_eq!(child_hash, _header.hash());
                            // Bump this one down to become a new root
                            let root_data = self.enroll_root(child_hash);
                            let _old = self.roots.insert(child_hash, root_data);
                            debug_assert!(_old.is_none());
                        }
                        CandidateData::Block(_block) => {
                            debug_assert_eq!(child_hash, _block.header().hash());
                            // Extract the block and descend to children
                            let candidate = entry.remove();
                            if let CandidateData::Block(block) = candidate.data {
                                block_avalanche.push(block);
                            } else {
                                unsafe { unreachable_unchecked() }
                            }
                            child_hashes.extend(candidate.children);
                        }
                    },
                    Vacant(_) => panic!("referential integrity failure in CandidateForest"),
                }
            }
        } else {
            assert!(
                !self.candidate_map.contains_key(&block_hash),
                "missed when a chain pull root candidate got committed to storage",
            );
        }
        block_avalanche
    }

    // Removes the root from, then walks up the tree and
    // removes all the descendant candidates.
    fn expunge_root(&mut self, root_hash: HeaderHash) {
        self.remove_root(&root_hash);
        let mut hashes = vec![root_hash];
        while let Some(hash) = hashes.pop() {
            let candidate = self
                .candidate_map
                .remove(&hash)
                .expect("referential integrity failure in CandidateForest");
            hashes.extend(candidate.children);
        }
    }

    fn poll_purge(&mut self) -> Poll<(), timer::Error> {
        loop {
            match self.expirations.poll()? {
                Async::NotReady => {
                    // Nothing to process now.
                    // Return Ready to release the lock.
                    return Ok(Async::Ready(()));
                }
                Async::Ready(None) => return Ok(Async::Ready(())),
                Async::Ready(Some(entry)) => {
                    self.expunge_root(entry.into_inner());
                }
            }
        }
    }
}
