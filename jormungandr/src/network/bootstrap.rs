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
use tokio::prelude::future::Either;
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

    let bootstrap = grpc::connect(peer.address(), None, runtime.executor())
        .map_err(|e| Error::Connect { source: e })
        .and_then(|client: Connection<BlockConfig>| {
            client
                .ready()
                .map_err(|e| Error::ClientNotReady { source: e })
        })
        .join(branch.get_ref())
        .and_then(move |(mut client, tip)| {
            let tip_hash = tip.hash();
            debug!(logger, "pulling blocks starting from {}", tip_hash);
            client
                .pull_blocks_to_tip(&[tip_hash])
                .map_err(|e| Error::PullRequestFailed { source: e })
                .and_then(move |stream| {
                    bootstrap_from_stream(blockchain, branch, tip, stream, logger)
                })
        });

    runtime.block_on_all(bootstrap)
}

fn bootstrap_from_stream<S>(
    blockchain: Blockchain,
    branch: Tip,
    tip: Arc<Ref>,
    stream: S,
    logger: Logger,
) -> impl Future<Item = Arc<Ref>, Error = Error>
where
    S: Stream<Item = Block, Error = NetworkError>,
    S::Error: Debug,
{
    let block0 = blockchain.block0().clone();
    let logger2 = logger.clone();
    let blockchain2 = blockchain.clone();
    let branch2 = branch.clone();

    stream
        .skip_while(move |block| Ok(block.header.hash() == block0))
        .then(|res| Ok(res))
        .fold(tip, move |parent_tip, block_or_err| match block_or_err {
            Ok(block) => Either::A(handle_block(blockchain.clone(), block, logger.clone())),
            Err(e) => {
                let fut = blockchain::process_new_ref(
                    logger.clone(),
                    blockchain.clone(),
                    branch.clone(),
                    parent_tip.clone(),
                )
                .then(|_| Err(Error::PullStreamFailed { source: e }));
                Either::B(fut)
            }
        })
        .and_then(move |new_tip| {
            blockchain::process_new_ref(logger2, blockchain2, branch2, new_tip.clone())
                .map_err(|e| Error::ChainSelectionFailed { source: e })
                .map(|()| new_tip)
        })
}

fn handle_block(
    blockchain: Blockchain,
    block: Block,
    logger: Logger,
) -> impl Future<Item = Arc<Ref>, Error = Error> {
    let header = block.header();
    let end_blockchain = blockchain.clone();
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
            debug!(
                logger,
                "validated block";
                "hash" => %post_checked.header().hash(),
                "block_date" => %post_checked.header().block_date(),
            );
            end_blockchain
                .apply_and_store_block(post_checked, block)
                .map(|applied| applied.expect("validated block must be unique"))
                .map_err(|e| Error::ApplyBlockFailed { source: e })
        })
}
