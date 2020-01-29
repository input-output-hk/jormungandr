use super::{
    chain::{self, Blockchain, HeaderChainVerifyError, PreCheckedHeader},
    chunk_sizes,
};
use crate::blockcfg::{Header, HeaderHash};
use crate::utils::async_msg::MessageQueue;

use futures::future::{self, Either, Loop};
use futures::prelude::*;
use slog::Logger;

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
    parent_header: Header,
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
    fn process_header(&mut self, header: Header) -> Result<(), Error> {
        // Pre-validate the chain and pick up header hashes.
        let block_hash = header.hash();
        let parent_hash = header.block_parent_hash();
        if parent_hash != self.parent_header.hash() {
            return Err(Error::BrokenHeaderChain(parent_hash));
        }
        // TODO: replace with a Blockchain method call
        // when that can pre-validate headers without
        // up-to-date ledger.
        chain::pre_verify_link(&header, &self.parent_header)?;
        debug!(
            self.logger,
            "adding block to fetch";
            "hash" => %block_hash,
            "parent" => %parent_hash,
        );
        self.new_hashes.push(block_hash);
        self.parent_header = header;
        Ok(())
    }

    fn poll_done(&mut self) -> Poll<chain_advance::Outcome, Error> {
        use self::chain_advance::Outcome;

        loop {
            if let Some(header) = self.header.take() {
                self.process_header(header)?;
            } else {
                match try_ready!(self.stream.poll().map_err(|()| Error::Unexpected)) {
                    Some(header) => {
                        self.process_header(header)?;
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

fn land_header_chain(
    blockchain: Blockchain,
    stream: HeaderStream,
    logger: Logger,
) -> impl Future<Item = Option<ChainAdvance>, Error = Error> {
    chain_landing::State::start(stream.map_err(|()| unreachable!()), blockchain)
        .and_then(move |state| state.skip_present_blocks())
        .and_then(move |maybe_new| match maybe_new {
            Some((header, stream)) => {
                // We have got a header that may not be in storage yet,
                // but its parent is.
                // Find an existing root or create a new one.
                let root_hash = header.hash();
                let root_parent_hash = header.block_parent_hash();
                debug!(
                    logger,
                    "landed the header chain";
                    "hash" => %root_hash,
                    "parent" => %root_parent_hash,
                );
                let new_hashes = vec![root_hash];
                let landing = ChainAdvance {
                    stream: stream.into_inner(),
                    parent_header: header,
                    header: None,
                    new_hashes,
                    logger,
                };
                future::ok(Some(landing))
            }
            None => {
                debug!(logger, "all blocks already present for the header chain");
                future::ok(None)
            }
        })
}

/// Consumes headers from the stream, filtering out those that are already
/// present and validating the chain integrity for the remainder.
/// Returns a future that resolves to a batch of block hashes to request
/// from the network,
/// and the stream if the process terminated early due to reaching
/// a limit on the number of blocks or (TODO: implement) needing
/// block data to validate more blocks with newer leadership information.
pub fn advance_branch(
    blockchain: Blockchain,
    header_stream: HeaderStream,
    logger: Logger,
) -> impl Future<Item = (Vec<HeaderHash>, Option<HeaderStream>), Error = Error> {
    land_header_chain(blockchain, header_stream, logger).and_then(move |mut advance| {
        if advance.is_some() {
            let fut = future::poll_fn(move || {
                use self::chain_advance::Outcome;
                let done = try_ready!(advance.as_mut().unwrap().poll_done());
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
