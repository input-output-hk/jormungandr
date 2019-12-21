use super::{
    chain::{self, Blockchain, HeaderChainVerifyError, PreCheckedHeader},
    chunk_sizes,
};
use crate::blockcfg::{Block, Header, HeaderHash};
use crate::utils::async_msg::MessageQueue;
use chain_core::property::{Block as _, HasHeader};

use futures::future::{self, Either, Loop};
use futures::prelude::*;
use slog::Logger;
use tokio::sync::lock::Lock;
use tokio::timer::{self, delay_queue, DelayQueue};

use std::collections::HashMap;
use std::hint::unreachable_unchecked;
use std::time::Duration;

// derive
use thiserror::Error;

type HeaderStream = MessageQueue<Header>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("the incoming header stream is empty")]
    EmptyHeaderStream,
    #[error("header chain verification failed")]
    Blockchain(#[from] chain::Error),
    #[error("the parent block {0} of the first received block header is not found in storage")]
    MissingParentBlock(HeaderHash),
    #[error("the parent hash field {0} of a received block header does not match the hash of the preceding header")]
    BrokenHeaderChain(HeaderHash),
    // FIXME: this needs to be merged into the Blockchain variant above
    // when Blockchain can pre-validate headers without up-to-date ledger.
    #[error("block headers do not form a valid chain: {0}")]
    HeaderChainVerificationFailed(#[from] HeaderChainVerifyError),
    #[error("unexpected header stream failure")]
    Unexpected,
}

#[derive(Clone)]
pub struct CandidateForest {
    inner: Lock<CandidateForestThickets>,
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
    Applied(Header),
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

    fn applied(header: Header) -> Self {
        Candidate {
            data: CandidateData::Applied(header),
            children: Vec::new(),
        }
    }

    fn has_block(&self) -> bool {
        use self::CandidateData::*;
        match self.data {
            Header(_) => false,
            Block(_) => true,
            Applied(_) => false,
        }
    }

    fn is_applied(&self) -> bool {
        use self::CandidateData::*;
        match self.data {
            Applied(_) => true,
            _ => false,
        }
    }

    // FIXME: fix the clone happiness, comes from chain_core::property::HasHeader
    fn header(&self) -> Header {
        use self::CandidateData::*;
        match &self.data {
            Header(header) => header.clone(),
            Block(block) => block.header(),
            Applied(header) => header.clone(),
        }
    }
}

struct RootData {
    expiration_key: delay_queue::Key,
}

mod chain_landing {
    use super::*;

    pub struct State<S> {
        blockchain: Blockchain,
        header: Header,
        stream: S,
    }

    impl<S> State<S>
    where
        S: Stream<Item = Header, Error = Error>,
    {
        /// Read the first header from the stream.
        /// Return a future that resolves to a state object.
        /// This method starts the sequence of processing a header chain.
        pub fn start(stream: S, blockchain: Blockchain) -> impl Future<Item = Self, Error = Error> {
            stream
                .into_future()
                .map_err(|(err, _)| err)
                .and_then(move |(maybe_first, stream)| match maybe_first {
                    Some(header) => {
                        let state = State {
                            blockchain,
                            header,
                            stream,
                        };
                        Ok(state)
                    }
                    None => Err(Error::EmptyHeaderStream),
                })
        }

        /// Reads the stream and skips blocks that are already present in the storage.
        /// Resolves with the header of the first block that is not present,
        /// but its parent is in storage, and the stream with headers remaining
        /// to be read. If the stream ends before the requisite header is found,
        /// resolves with None.
        /// The chain also is pre-verified for sanity.
        pub fn skip_present_blocks(self) -> impl Future<Item = Option<(Header, S)>, Error = Error> {
            future::loop_fn(self, move |state| {
                let State {
                    blockchain,
                    header,
                    stream,
                } = state;
                blockchain
                    .pre_check_header(header, false)
                    .map_err(|e| e.into())
                    .and_then(move |pre_checked| match pre_checked {
                        PreCheckedHeader::AlreadyPresent { .. } => {
                            let fut = stream.into_future().map_err(|(err, _)| err).and_then(
                                move |(maybe_next, stream)| match maybe_next {
                                    Some(header) => {
                                        let state = State {
                                            blockchain,
                                            header,
                                            stream,
                                        };
                                        Ok(Loop::Continue(state))
                                    }
                                    None => Ok(Loop::Break(None)),
                                },
                            );
                            Either::A(fut)
                        }
                        PreCheckedHeader::HeaderWithCache { header, .. } => {
                            Either::B(future::ok(Loop::Break(Some((header, stream)))))
                        }
                        PreCheckedHeader::MissingParent { header } => Either::B(future::err(
                            Error::MissingParentBlock(header.block_parent_hash()),
                        )),
                    })
            })
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
                        .ok_or(Error::MissingParentBlock(parent_hash.clone()))?;
                    // TODO: replace with a Blockchain method call
                    // when that can pre-validate headers without
                    // up-to-date ledger.
                    chain::pre_verify_link(&header, &parent_candidate.header())?;
                    if parent_candidate.is_applied() {
                        // The parent block has been committed to storage
                        // before this header was received.
                        // Drop the block hashes collected for fetching so far
                        // and try to re-land the chain.
                        self.new_hashes.clear();
                        let (_, is_new) = forest.add_or_refresh_root(header);
                        debug!(
                            self.logger,
                            "re-landed the header chain, {}",
                            if is_new { "new root" } else { "existing root" };
                            "hash" => %block_hash,
                            "parent" => %parent_hash,
                        );
                        if is_new {
                            self.new_hashes.push(block_hash);
                        }
                    } else {
                        debug_assert!(!parent_candidate.children.contains(&block_hash));
                        parent_candidate.children.push(block_hash);
                        forest
                            .candidate_map
                            .insert(block_hash, Candidate::from_header(header));
                        debug!(
                            self.logger,
                            "adding block to fetch";
                            "hash" => %block_hash,
                            "parent" => %parent_hash,
                        );
                        self.new_hashes.push(block_hash);
                    }
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
            if self.new_hashes.len() as u64 >= chunk_sizes::BLOCKS {
                return Ok(Outcome::Incomplete.into());
            }
        }
    }
}

impl CandidateForest {
    pub fn new(root_ttl: Duration, logger: Logger) -> Self {
        let inner = CandidateForestThickets {
            candidate_map: HashMap::new(),
            roots: HashMap::new(),
            expirations: DelayQueue::new(),
            root_ttl,
        };
        CandidateForest {
            inner: Lock::new(inner),
            logger,
        }
    }

    fn land_header_chain(
        &self,
        blockchain: Blockchain,
        stream: HeaderStream,
    ) -> impl Future<Item = Option<ChainAdvance>, Error = Error> {
        let mut inner = self.inner.clone();
        let logger = self.logger.clone();

        chain_landing::State::start(stream.map_err(|()| unreachable!()), blockchain)
            .and_then(move |state| state.skip_present_blocks())
            .and_then(move |maybe_new| match maybe_new {
                Some((header, stream)) => {
                    // We have got a header that may not be in storage yet,
                    // but its parent is.
                    // Find an existing root or create a new one.
                    let fut = future::poll_fn(move || Ok(inner.poll_lock())).and_then(
                        move |mut forest| {
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
                            Ok(Some(landing))
                        },
                    );
                    Either::A(fut)
                }
                None => Either::B(future::ok(None)),
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
        blockchain: Blockchain,
        header_stream: HeaderStream,
    ) -> impl Future<Item = (Vec<HeaderHash>, Option<HeaderStream>), Error = Error> {
        let mut inner = self.inner.clone();
        self.land_header_chain(blockchain, header_stream)
            .and_then(move |mut advance| {
                if advance.is_some() {
                    let fut = future::poll_fn(move || {
                        use self::chain_advance::Outcome;
                        let done = try_ready!(advance.as_mut().unwrap().poll_done(&mut inner));
                        let advance = advance.take().unwrap();
                        let ret_stream = match done {
                            Outcome::Complete => None,
                            Outcome::Incomplete => Some(advance.stream),
                        };
                        Ok((advance.new_hashes, ret_stream).into())
                    });
                    Either::A(fut)
                } else {
                    Either::B(future::ok((Vec::new(), None)))
                }
            })
    }

    /// Puts a block into the cache for later application.
    ///
    /// The block's header must have been earlier registered in a header chain
    /// passed to the `advance_branch` method. If the block is already
    /// in the cache, the block value is not updated and the returned future
    /// resolves successfully.
    pub fn cache_block(&self, block: Block) -> impl Future<Item = (), Error = chain::Error> {
        let block_hash = block.id();
        let mut inner = self.inner.clone();
        future::poll_fn(move || Ok(inner.poll_lock())).and_then(move |mut forest| {
            forest
                .cache_requested_block(block_hash, block)
                .map_err(|_block| chain::ErrorKind::BlockNotRequested(block_hash).into())
        })
    }

    pub fn apply_block(
        &self,
        block: Block,
    ) -> impl Future<Item = Vec<Block>, Error = chain::Error> {
        let mut inner = self.inner.clone();
        future::poll_fn(move || Ok(inner.poll_lock()))
            .and_then(move |mut forest| Ok(forest.apply_block(block)))
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
                debug_assert!(
                    _old.is_none(),
                    "chain pull root candidate {} was previously cached",
                    root_hash,
                );
                true
            }
            Occupied(entry) => {
                debug_assert!(
                    !self
                        .candidate_map
                        .get(&root_hash)
                        .expect("chain pull root candidate should be in the map")
                        .has_block(),
                    "a chain pull root candidate should not cache a block",
                );
                self.expirations
                    .reset(&entry.get().expiration_key, self.root_ttl);
                false
            }
        };
        (root_hash, is_new)
    }

    fn apply_candidate(&mut self, block_hash: HeaderHash) -> Candidate {
        use std::collections::hash_map::Entry::*;

        match self.candidate_map.entry(block_hash) {
            Occupied(mut entry) => {
                let header = entry.get().header();
                entry.insert(Candidate::applied(header))
            }
            Vacant(_) => panic!("referential integrity failure in CandidateForest"),
        }
    }

    fn apply_block(&mut self, block: Block) -> Vec<Block> {
        use std::collections::hash_map::Entry::*;

        let block_hash = block.id();
        if self.roots.contains_key(&block_hash) {
            let candidate = self.apply_candidate(block_hash);
            debug_assert!(
                !candidate.has_block(),
                "a chain pull root candidate should not cache a block",
            );
            let mut block_avalanche = vec![block];
            let mut child_hashes = candidate.children;
            while let Some(child_hash) = child_hashes.pop() {
                match self.candidate_map.entry(child_hash) {
                    Occupied(mut entry) => {
                        // Promote the child to a new root entry
                        let expiration_key = self.expirations.insert(child_hash, self.root_ttl);
                        let root_data = RootData { expiration_key };
                        let _old = self.roots.insert(child_hash, root_data);
                        debug_assert!(_old.is_none());
                        match &entry.get().data {
                            CandidateData::Header(_header) => {
                                debug_assert_eq!(child_hash, _header.hash());
                            }
                            CandidateData::Block(block) => {
                                let header = block.header();
                                debug_assert_eq!(child_hash, header.hash());
                                // Extract the block and descend to children
                                let candidate = entry.insert(Candidate::applied(header));
                                if let CandidateData::Block(block) = candidate.data {
                                    block_avalanche.push(block);
                                } else {
                                    unsafe { unreachable_unchecked() }
                                }
                                child_hashes.extend(candidate.children);
                            }
                            CandidateData::Applied(_) => {
                                panic!("a child block has been applied ahead of the parent")
                            }
                        }
                    }
                    Vacant(_) => panic!("referential integrity failure in CandidateForest"),
                }
            }
            block_avalanche
        } else {
            match self.cache_requested_block(block_hash, block) {
                Ok(()) => {
                    // The task that applies the block has won the lock before
                    // other tasks that should apply preceding blocks.
                    // The block is cached for later, return an empty vector.
                    Vec::default()
                }
                Err(block) => {
                    // The block is not part of a chain pull.
                    // Pass it through so that it gets applied to storage
                    // or fails to validate against the parent that should be
                    // already stored.
                    vec![block]
                }
            }
        }
    }

    fn cache_requested_block(&mut self, block_hash: HeaderHash, block: Block) -> Result<(), Block> {
        use std::collections::hash_map::Entry::*;

        match self.candidate_map.entry(block_hash) {
            Vacant(_) => Err(block),
            Occupied(mut entry) => {
                match &entry.get().data {
                    CandidateData::Header(header) => {
                        debug_assert!(header.hash() == block_hash);
                        entry.insert(Candidate::from_block(block));
                    }
                    CandidateData::Block(block) => {
                        debug_assert!(block.header().hash() == block_hash);
                    }
                    CandidateData::Applied(header) => {
                        debug_assert!(header.hash() == block_hash);
                    }
                }
                Ok(())
            }
        }
    }

    // Removes the root from, then walks up the tree and
    // removes all the descendant candidates.
    fn expunge_root(&mut self, root_hash: HeaderHash) {
        self.roots.remove(&root_hash);
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
