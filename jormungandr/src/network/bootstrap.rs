use super::{grpc, BlockConfig};
use crate::blockcfg::{Block, HeaderHash};
use crate::blockchain::{self, Blockchain, Error as BlockchainError, PreCheckedHeader, Ref, Tip};
use crate::settings::start::network::Peer;
use chain_core::property::HasHeader;
use network_core::client::{BlockService, Client as _};
use network_core::error::Error as NetworkError;
use network_grpc::client::Connection;
use slog::Logger;
use thiserror::Error;
use tokio::prelude::*;
use tokio::runtime::Runtime;

use std::fmt::Debug;
use std::io;
use std::sync::Arc;

#[derive(Error, Debug)]
pub enum Error {
    #[error("runtime initialization failed")]
    RuntimeInit { source: io::Error },
    #[error("failed to connect to bootstrap peer")]
    Connect { source: grpc::ConnectError },
    #[error("connection broken")]
    ClientNotReady { source: NetworkError },
    #[error("bootstrap pull request failed")]
    PullRequestFailed { source: NetworkError },
    #[error("bootstrap pull stream failed")]
    PullStreamFailed { source: NetworkError },
    #[error("block header check failed")]
    HeaderCheckFailed { source: BlockchainError },
    #[error("received block {0} is already present")]
    BlockAlreadyPresent(HeaderHash),
    #[error("received block {0} is not connected to the block chain")]
    BlockMissingParent(HeaderHash),
    #[error("failed to apply block to the blockchain")]
    ApplyBlockFailed { source: BlockchainError },
    #[error("failed to select the new tip")]
    ChainSelectionFailed { source: BlockchainError },
}

pub fn bootstrap_from_peer(
    peer: Peer,
    blockchain: Blockchain,
    branch: Tip,
    logger: Logger,
) -> Result<Arc<Ref>, Error> {
    info!(logger, "connecting to bootstrap peer {}", peer.connection);

    let runtime = Runtime::new().map_err(|e| Error::RuntimeInit { source: e })?;

    let blockchain2 = blockchain.clone();
    let logger2 = logger.clone();

    let bootstrap = grpc::connect(peer.address(), None, runtime.executor())
        .map_err(|e| Error::Connect { source: e })
        .and_then(|client: Connection<BlockConfig>| {
            client
                .ready()
                .map_err(|e| Error::ClientNotReady { source: e })
        })
        .join(branch.get_ref().map_err(|_| unreachable!()))
        .and_then(move |(mut client, tip)| {
            let tip_hash = tip.hash();
            debug!(logger, "pulling blocks starting from {}", tip_hash);
            client
                .pull_blocks_to_tip(&[tip_hash])
                .map_err(|e| Error::PullRequestFailed { source: e })
                .and_then(move |stream| bootstrap_from_stream(blockchain, tip, stream, logger))
        })
        .and_then(move |tip| {
            blockchain::process_new_ref(logger2, blockchain2, branch, tip.clone())
                .map_err(|e| Error::ChainSelectionFailed { source: e })
                .map(|()| tip)
        });

    runtime.block_on_all(bootstrap)
}

fn bootstrap_from_stream<S>(
    blockchain: Blockchain,
    tip: Arc<Ref>,
    stream: S,
    logger: Logger,
) -> impl Future<Item = Arc<Ref>, Error = Error>
where
    S: Stream<Item = Block, Error = NetworkError>,
    S::Error: Debug,
{
    let fold_logger = logger.clone();
    stream
        .map_err(|e| Error::PullStreamFailed { source: e })
        .fold(tip, move |_, block| {
            handle_block(blockchain.clone(), block, fold_logger.clone())
        })
}

fn handle_block(
    mut blockchain: Blockchain,
    block: Block,
    logger: Logger,
) -> impl Future<Item = Arc<Ref>, Error = Error> {
    let header = block.header();
    trace!(
        logger,
        "received block from the bootstrap node: {:#?}",
        header
    );
    let mut end_blockchain = blockchain.clone();
    blockchain
        .pre_check_header(header, true)
        .map_err(|e| Error::HeaderCheckFailed { source: e })
        .and_then(|pre_checked| match pre_checked {
            PreCheckedHeader::AlreadyPresent { header, .. } => {
                Err(Error::BlockAlreadyPresent(header.hash()))
            }
            PreCheckedHeader::MissingParent { header, .. } => {
                Err(Error::BlockMissingParent(header.hash()))
            }
            PreCheckedHeader::HeaderWithCache { header, parent_ref } => Ok((header, parent_ref)),
        })
        .and_then(move |(header, parent_ref)| {
            blockchain
                .post_check_header(header, parent_ref)
                .map_err(|e| Error::HeaderCheckFailed { source: e })
        })
        .and_then(move |post_checked| {
            end_blockchain
                .apply_and_store_block(post_checked, block)
                .map_err(|e| Error::ApplyBlockFailed { source: e })
        })
}
