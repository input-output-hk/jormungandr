use super::{Error, ErrorKind, Storage};
use crate::blockcfg::{Block, Header, HeaderHash};
use crate::utils::async_msg::MessageQueue;
use chain_core::property::{ChainLength as _, HasHeader};

use futures::future::{self, Either, Loop};
use futures::prelude::*;
use futures::stream;
use tokio::sync::lock::Lock;
use tokio::timer;

use std::collections::{HashMap, VecDeque};
use std::convert::Infallible;
use std::time::{Duration, Instant};

#[derive(Clone)]
pub struct CandidateRepo {
    block_cache: BlockCache<Candidate>,
    branches: CandidateBranches,
    storage: Storage,
}

#[derive(Clone)]
enum Candidate {
    Header(Header),
    Block(Block),
}

struct CandidateBranch {
    hashes: VecDeque<HeaderHash>,
    last_updated: Instant,
}

#[derive(Clone)]
struct CandidateBranches {
    inner: Lock<HashMap<HeaderHash, CandidateBranch>>,
}

struct SplicedHeaderChain {
    branch: CandidateBranch,
    parent_header: Header,
    headers: Vec<Header>,
}

impl CandidateBranch {
    fn empty() -> Self {
        CandidateBranch {
            hashes: VecDeque::new(),
            last_updated: Instant::now(),
        }
    }

    fn tip_hash(&self) -> HeaderHash {
        self.hashes
            .back()
            .expect("branch must not be empty")
            .clone()
    }

    fn push(&mut self, hash: HeaderHash) {
        self.hashes.push_back(hash)
    }
}

impl CandidateBranches {
    fn new() -> Self {
        CandidateBranches {
            inner: Lock::new(HashMap::new()),
        }
    }

    fn take_branch(
        &self,
        block_id: HeaderHash,
    ) -> impl Future<Item = Option<CandidateBranch>, Error = Infallible> {
        let mut branches = self.inner.clone();
        future::poll_fn(move || Ok(branches.poll_lock()))
            .map(move |mut branch_map| branch_map.remove(&block_id))
    }

    fn set_branch(
        &self,
        mut branch: CandidateBranch,
    ) -> impl Future<Item = (), Error = Infallible> {
        let block_id = branch.tip_hash();
        let mut branches = self.inner.clone();
        future::poll_fn(move || Ok(branches.poll_lock())).map(move |mut branch_map| {
            branch.last_updated = Instant::now();
            branch_map.insert(block_id, branch);
        })
    }
}

impl CandidateRepo {
    pub fn new(storage: Storage, ref_cache_ttl: Duration) -> Self {
        CandidateRepo {
            block_cache: BlockCache::new(ref_cache_ttl),
            branches: CandidateBranches::new(),
            storage: storage.clone(),
        }
    }

    pub fn get_block(
        &mut self,
        header_hash: HeaderHash,
    ) -> impl Future<Item = Option<Block>, Error = Error> {
        let storage = self.storage.clone();
        self.block_cache
            .get(header_hash.clone())
            .map_err(|_: Infallible| unreachable!())
            .and_then(move |maybe_entry| match maybe_entry {
                None => Either::A(storage.get(header_hash).map_err(|e| e.into())),
                Some(Candidate::Header(_header)) => {
                    // FIXME: this should be an error as get_block should only
                    // be used to extract previously committed blocks.
                    Either::B(future::ok(None))
                }
                Some(Candidate::Block(block)) => Either::B(future::ok(Some(block))),
            })
    }

    fn splice_headers(
        &self,
        header_stream: MessageQueue<Header>,
    ) -> impl Future<Item = Option<SplicedHeaderChain>, Error = Error> {
        struct State {
            headers: Vec<Header>,
            index: usize,
            branches: CandidateBranches,
            block_cache: BlockCache<Candidate>,
            storage: Storage,
        }

        // Skip headers of blocks already present in the storage, then
        // try to resolve the branch.
        future::loop_fn(
            State {
                headers,
                index: 0,
                branches: self.branches.clone(),
                block_cache: self.block_cache.clone(),
                storage: self.storage.clone(),
            },
            move |state| {
                if state.index >= state.headers.len() {
                    return Either::A(future::ok(Loop::Break(state)));
                }
                let header = &state.headers[state.index];
                Either::B(
                    state
                        .storage
                        .block_exists(header.hash())
                        .map_err(|e| e.into())
                        .map(move |exists| {
                            if exists {
                                Loop::Continue(State {
                                    index: state.index + 1,
                                    ..state
                                })
                            } else {
                                Loop::Break(state)
                            }
                        }),
                )
            },
        )
        .and_then(|mut state| {
            if state.index >= state.headers.len() {
                return Either::A(future::ok(None));
            }
            let headers = state.headers.split_off(state.index);
            // Locate the branch to splice. If not found, there has
            // to be the parent block in the storage to start the branch from.
            let parent_block_id = headers[0].block_parent_hash().clone();
            Either::B(
                state
                    .branches
                    .take_branch(parent_block_id)
                    .map_err(|_: Infallible| unreachable!())
                    .and_then(move |maybe_branch| match maybe_branch {
                        Some(branch) => Either::A(
                            state
                                .block_cache
                                .get(parent_block_id)
                                .map_err(|_: Infallible| unreachable!())
                                .and_then(move |maybe_entry| {
                                    let entry = maybe_entry.expect("block must be in cache");
                                    let parent_header = match entry {
                                        Candidate::Header(header) => header.clone(),
                                        Candidate::Block(block) => block.header(),
                                    };
                                    Ok(Some(SplicedHeaderChain {
                                        branch,
                                        parent_header,
                                        headers,
                                    }))
                                }),
                        ),
                        None => Either::B(
                            state
                                .storage
                                .get(parent_block_id)
                                .map_err(|e| e.into())
                                .and_then(move |maybe_block| {
                                    let branch = CandidateBranch::empty();
                                    match maybe_block {
                                        Some(block) => Ok(Some(SplicedHeaderChain {
                                            branch,
                                            parent_header: block.header(),
                                            headers,
                                        })),
                                        None => Err(ErrorKind::BlockHeaderMissingParent(
                                            parent_block_id,
                                        )
                                        .into()),
                                    }
                                }),
                        ),
                    }),
            )
        })
    }

    pub fn advance_branch(
        &self,
        header_stream: MessageQueue<Header>,
    ) -> impl Future<Item = Vec<HeaderHash>, Error = Error> {
        let branches = self.branches.clone();
        let block_cache = self.block_cache.clone();
        self.splice_headers(headers)
            .and_then(move |maybe_spliced| {
                match maybe_spliced {
                    None => Either::A(future::ok(Vec::new())),
                    Some(SplicedHeaderChain {
                        mut branch,
                        parent_header: parent,
                        headers
                    }) => {
                        let block_ids = headers
                            .iter()
                            .map(|header| header.hash())
                            .collect::<Vec<_>>();
                        for header in &headers {
                            branch.push(header.hash());
                        }
                        let cache_headers = stream::iter_ok(headers)
                            .fold(parent, move |parent, header| {
                                // TODO: reuse validation code
                                if header.block_parent_hash() != &parent.hash() {
                                    return Either::A(future::err(Error::from_kind(ErrorKind::BlockHeaderVerificationFailed(
                                        "parent hash of a block does not match the preceding header"
                                            .into(),
                                    ))));
                                }
                                if header.block_date() <= parent.block_date() {
                                    return Either::A(future::err(Error::from_kind(ErrorKind::BlockHeaderVerificationFailed(
                                        "block is not valid, date is set before parent's".into(),
                                    ))));
                                }
                                if header.chain_length() != parent.chain_length().next() {
                                    return Either::A(future::err(Error::from_kind(ErrorKind::BlockHeaderVerificationFailed(
                                        "block is not valid, chain length is not monotonically increasing"
                                            .into(),
                                    ))));
                                }
                                Either::B(
                                    block_cache
                                        .insert(header.hash(), Candidate::Header(header.clone()))
                                        .map_err(|_: Infallible| unreachable!())
                                        .map(move |()| header)
                                )
                            });
                        Either::B(
                            cache_headers
                                .and_then(move |_| {
                                    branches
                                        .set_branch(branch)
                                        .map_err(|_: Infallible| unreachable!())
                                })
                                .map(|()| block_ids)
                        )
                    }
                }
            })
    }

    /// Puts a block into the cache for later application.
    ///
    /// The block's header must have been earlier registered in a header chain
    /// passed to the `advance_branch` method. If the block is already
    /// in the cache, the block value is not updated and the returned future
    /// resolves successfully.
    pub fn cache_block(&self, block: Block) -> impl Future<Item = (), Error = Error> {
        let header = block.header();
        let block_id = header.hash();
        let block_cache = self.block_cache.clone();
        block_cache
            .get(block_id)
            .map_err(|_: Infallible| unreachable!())
            .and_then(move |maybe_candidate| match maybe_candidate {
                Some(Candidate::Header(header)) => {
                    debug_assert!(header.hash() == block_id);
                    Either::A(
                        block_cache
                            .insert(block_id, Candidate::Block(block))
                            .map_err(|_: Infallible| unreachable!()),
                    )
                }
                Some(Candidate::Block(block)) => {
                    debug_assert!(block.header().hash() == block_id);
                    Either::B(future::ok(()))
                }
                None => Either::B(future::err(ErrorKind::MissingParentBlock(header).into())),
            })
    }

    /// return a future that will remove every expired branch from the cache.
    pub fn purge(&self) -> impl Future<Item = (), Error = timer::Error> {
        // TODO: purge expired branches
        self.block_cache.purge()
    }
}
