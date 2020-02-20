use super::{grpc, BlockConfig};
use crate::blockcfg::{Block, HeaderHash};
use crate::blockchain::{self, Blockchain, Error as BlockchainError, PreCheckedHeader, Ref, Tip};
use crate::settings::start::network::{Peer, Protocol};
use chain_core::property::HasHeader;
use network_core::client::{BlockService, Client as _, GossipService};
use network_core::error::Error as NetworkError;
use network_grpc::client::Connection;
use slog::Logger;
use thiserror::Error;
use tokio::prelude::future::Either;
use tokio::prelude::*;
use tokio_compat::prelude::*;
use tokio_compat::runtime::Runtime;

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
    #[error("peers not available broken")]
    PeersNotAvailable { source: NetworkError },
    #[error("bootstrap pull request failed")]
    PullRequestFailed { source: NetworkError },
    #[error("bootstrap pull stream failed")]
    PullStreamFailed { source: NetworkError },
    #[error("block header check failed")]
    HeaderCheckFailed { source: BlockchainError },
    #[error(
        "received block {0} is already present, but does not descend from any of the checkpoints"
    )]
    BlockNotOnBranch(HeaderHash),
    #[error("received block {0} is not connected to the block chain")]
    BlockMissingParent(HeaderHash),
    #[error("failed to fetch checkpoints from storage")]
    GetCheckpointsFailed { source: BlockchainError },
    #[error("failed to apply block to the blockchain")]
    ApplyBlockFailed { source: BlockchainError },
    #[error("failed to select the new tip")]
    ChainSelectionFailed { source: BlockchainError },
}

pub fn peers_from_trusted_peer(peer: &Peer, logger: Logger) -> Result<Vec<Peer>, Error> {
    info!(
        logger,
        "getting peers from bootstrap peer {}", peer.connection
    );

    let mut runtime = Runtime::new().map_err(|e| Error::RuntimeInit { source: e })?;
    let bootstrap = grpc::connect(peer.address(), None, runtime.executor())
        .map_err(|e| Error::Connect { source: e })
        .and_then(|client: Connection<BlockConfig>| {
            client
                .ready()
                .map_err(|e| Error::ClientNotReady { source: e })
        })
        .and_then(move |mut client| {
            client
                .peers()
                .map_err(|e| Error::PeersNotAvailable { source: e })
                .and_then(move |peers| {
                    info!(
                        logger,
                        "peer {} : peers known : {}",
                        peer.connection,
                        peers.peers.len()
                    );
                    let peers = peers
                        .peers
                        .iter()
                        .map(|peer| Peer::new(peer.addr, Protocol::Grpc))
                        .collect();
                    future::ok(peers)
                })
        });

    runtime.block_on(bootstrap)
}

pub fn bootstrap_from_peer(
    peer: Peer,
    blockchain: Blockchain,
    tip: Tip,
    logger: Logger,
) -> Result<(), Error> {
    info!(logger, "connecting to bootstrap peer {}", peer.connection);

    let mut runtime = Runtime::new().map_err(|e| Error::RuntimeInit { source: e })?;

    let bootstrap = grpc::connect(peer.address(), None, runtime.executor())
        .map_err(|e| Error::Connect { source: e })
        .and_then(|client: Connection<BlockConfig>| {
            client
                .ready()
                .map_err(|e| Error::ClientNotReady { source: e })
        })
        .join(
            blockchain
                .get_checkpoints(tip.branch())
                .map_err(|e| Error::GetCheckpointsFailed { source: e }),
        )
        .and_then(move |(mut client, checkpoints)| {
            debug!(
                logger,
                "pulling blocks starting from checkpoints: {:?}", checkpoints
            );
            client
                .pull_blocks_to_tip(checkpoints.as_slice())
                .map_err(|e| Error::PullRequestFailed { source: e })
                .and_then(move |stream| bootstrap_from_stream(blockchain, tip, stream, logger))
        });

    runtime.block_on(bootstrap)
}

fn bootstrap_from_stream<S>(
    blockchain: Blockchain,
    branch: Tip,
    stream: S,
    logger: Logger,
) -> impl Future<Item = (), Error = Error>
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
        .fold(
            None,
            move |parent_tip: Option<Arc<Ref>>, block_or_err| match block_or_err {
                Ok(block) => {
                    let fut = handle_block_old(blockchain.clone(), block, logger.clone()).map(Some);
                    Either::A(fut)
                }
                Err(e) => {
                    let fut = if let Some(parent_tip) = parent_tip {
                        Either::A(blockchain::process_new_ref(
                            logger.clone(),
                            blockchain.clone(),
                            branch.clone(),
                            parent_tip.clone(),
                        ))
                    } else {
                        Either::B(future::ok(()))
                    }
                    .then(|_| Err(Error::PullStreamFailed { source: e }));
                    Either::B(fut)
                }
            },
        )
        .and_then(move |maybe_new_tip| {
            if let Some(new_tip) = maybe_new_tip {
                Either::A(
                    blockchain::process_new_ref(logger2, blockchain2, branch2, new_tip.clone())
                        .map_err(|e| Error::ChainSelectionFailed { source: e }),
                )
            } else {
                info!(logger2, "no new blocks in bootstrap stream");
                Either::B(future::ok(()))
            }
        })
}

fn handle_block_old(
    blockchain: Blockchain,
    block: Block,
    logger: Logger,
) -> impl Future<Item = Arc<Ref>, Error = Error> {
    use futures_util::compat::Compat;
    Compat::new(Box::pin(handle_block(blockchain, block, logger)))
}

async fn handle_block(
    blockchain: Blockchain,
    block: Block,
    logger: Logger,
) -> Result<Arc<Ref>, Error> {
    let header = block.header();
    let pre_checked = blockchain
        .pre_check_header(header, true)
        .compat()
        .await
        .map_err(|e| Error::HeaderCheckFailed { source: e })?;
    match pre_checked {
        PreCheckedHeader::AlreadyPresent {
            cached_reference: Some(block_ref),
            ..
        } => Ok(block_ref),
        PreCheckedHeader::AlreadyPresent {
            cached_reference: None,
            header,
        } => Err(Error::BlockNotOnBranch(header.hash())),
        PreCheckedHeader::MissingParent { header, .. } => {
            Err(Error::BlockMissingParent(header.hash()))
        }
        PreCheckedHeader::HeaderWithCache { header, parent_ref } => {
            let post_checked = blockchain
                .post_check_header(header, parent_ref)
                .compat()
                .await
                .map_err(|e| Error::HeaderCheckFailed { source: e })?;

            debug!(
                logger,
                "validated block";
                "hash" => %post_checked.header().hash(),
                "block_date" => %post_checked.header().block_date(),
            );
            let applied = blockchain
                .apply_and_store_block(post_checked, block)
                .await
                .map_err(|e| Error::ApplyBlockFailed { source: e })?;
            Ok(applied.cached_ref()) //.map(|applied| applied.cached_ref())
        }
    }
}
