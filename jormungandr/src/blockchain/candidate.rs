use super::{
    chain::{self, Blockchain, HeaderChainVerifyError, PreCheckedHeader},
    chunk_sizes,
};
use crate::{
    blockcfg::{Header, HeaderHash},
    utils::async_msg::MessageQueue,
};
use futures::{
    future::poll_fn,
    prelude::*,
    ready,
    task::{Context, Poll},
};
use std::{marker::Unpin, pin::Pin};
// derive
use thiserror::Error;

type HeaderStream = MessageQueue<Header>;

#[allow(clippy::large_enum_variant)]
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
        S: Stream<Item = Header> + Unpin,
    {
        // Read the first header from the stream.
        // Return a future that resolves to a state object.
        // This method starts the sequence of processing a header chain.
        pub async fn start(stream: S, blockchain: Blockchain) -> Result<Self, Error> {
            let (maybe_first, stream) = stream.into_future().await;
            match maybe_first {
                Some(header) => Ok(State {
                    blockchain,
                    header,
                    stream,
                }),
                None => Err(Error::EmptyHeaderStream),
            }
        }

        /// Reads the stream and skips blocks that are already present in the storage.
        /// Resolves with the header of the first block that is not present,
        /// but its parent is in storage, and the stream with headers remaining
        /// to be read. If the stream ends before the requisite header is found,
        /// resolves with None.
        /// The chain also is pre-verified for sanity.
        pub async fn skip_present_blocks(self) -> Result<Option<(Header, S)>, Error> {
            let mut state = self;
            loop {
                let State {
                    blockchain,
                    header,
                    stream,
                } = state;

                let pre_checked = blockchain.pre_check_header(header, false).await?;

                match pre_checked {
                    PreCheckedHeader::AlreadyPresent { .. } => {
                        let (maybe_next, stream) = stream.into_future().await;
                        match maybe_next {
                            Some(header) => {
                                state = State {
                                    blockchain,
                                    header,
                                    stream,
                                };
                                continue;
                            }
                            None => break Ok(None),
                        }
                    }
                    PreCheckedHeader::HeaderWithCache { header, .. } => {
                        break Ok(Some((header, stream)))
                    }
                    PreCheckedHeader::MissingParent { header } => {
                        break Err(Error::MissingParentBlock(header.block_parent_hash()))
                    }
                }
            }
        }
    }
}

struct ChainAdvance<S>
where
    S: Stream<Item = Header> + Unpin,
{
    stream: S,
    parent_header: Header,
    header: Option<Header>,
    new_hashes: Vec<HeaderHash>,
}

mod chain_advance {
    pub enum Outcome {
        Incomplete,
        Complete,
    }
}

impl<S> ChainAdvance<S>
where
    S: Stream<Item = Header> + Unpin,
{
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
        tracing::debug!(
            hash = %block_hash,
            parent = %parent_hash,
            "adding block to fetch"
        );
        self.new_hashes.push(block_hash);
        self.parent_header = header;
        Ok(())
    }

    fn poll_done(&mut self, cx: &mut Context) -> Poll<Result<chain_advance::Outcome, Error>> {
        use self::chain_advance::Outcome;

        loop {
            if let Some(header) = self.header.take() {
                self.process_header(header)?;
            } else {
                match ready!(Pin::new(&mut self.stream).poll_next(cx)) {
                    Some(header) => {
                        self.process_header(header)?;
                    }
                    None => return Poll::Ready(Ok(Outcome::Complete)),
                }
            }
            // TODO: bail out when block data are needed due to new epoch.
            if self.new_hashes.len() as u64 >= chunk_sizes::BLOCKS {
                return Poll::Ready(Ok(Outcome::Incomplete));
            }
        }
    }
}

async fn land_header_chain<S>(
    blockchain: Blockchain,
    stream: S,
) -> Result<Option<ChainAdvance<S>>, Error>
where
    S: Stream<Item = Header> + Unpin,
{
    let state = chain_landing::State::start(stream, blockchain).await?;
    let maybe_new = state.skip_present_blocks().await?;
    match maybe_new {
        Some((header, stream)) => {
            // We have got a header that may not be in storage yet,
            // but its parent is.
            // Find an existing root or create a new one.
            let root_hash = header.hash();
            let root_parent_hash = header.block_parent_hash();
            tracing::debug!(
                hash = %root_hash,
                parent = %root_parent_hash,
                "landed the header chain"
            );
            let new_hashes = vec![root_hash];
            let landing = ChainAdvance {
                stream,
                parent_header: header,
                header: None,
                new_hashes,
            };
            Ok(Some(landing))
        }
        None => {
            tracing::debug!("all blocks already present for the header chain");
            Ok(None)
        }
    }
}

/// Consumes headers from the stream, filtering out those that are already
/// present and validating the chain integrity for the remainder.
/// Returns a future that resolves to a batch of block hashes to request
/// from the network,
/// and the stream if the process terminated early due to reaching
/// a limit on the number of blocks or (TODO: implement) needing
/// block data to validate more blocks with newer leadership information.
pub async fn advance_branch(
    blockchain: Blockchain,
    header_stream: HeaderStream,
) -> Result<(Vec<HeaderHash>, Option<impl Stream<Item = Header>>), Error> {
    let mut advance = land_header_chain(blockchain, header_stream).await?;

    if advance.is_some() {
        poll_fn(|cx| {
            use self::chain_advance::Outcome;
            let done = ready!(advance.as_mut().unwrap().poll_done(cx));
            let advance = advance.take().unwrap();
            let ret_stream = match done {
                Ok(Outcome::Complete) => None,
                Ok(Outcome::Incomplete) => Some(advance.stream),
                Err(err) => return Poll::Ready(Err(err)),
            };
            Poll::Ready(Ok((advance.new_hashes, ret_stream)))
        })
        .await
    } else {
        Ok((Vec::new(), None))
    }
}
