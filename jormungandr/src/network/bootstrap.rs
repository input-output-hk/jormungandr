use super::grpc;
use crate::blockcfg::{Block, HeaderDesc, HeaderHash};
use crate::blockchain::{self, Blockchain, Error as BlockchainError, PreCheckedHeader, Ref, Tip};
use crate::settings::start::network::Peer;
use chain_core::property::{Deserialize, HasHeader};
use chain_network::data as net_data;
use chain_network::error::Error as NetworkError;
use futures03::prelude::*;
use slog::Logger;

use std::fmt::Debug;
use std::io;
use std::sync::Arc;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("runtime initialization failed")]
    RuntimeInit { source: io::Error },
    #[error("failed to connect to bootstrap peer")]
    Connect { source: grpc::ConnectError },
    #[error("peers not available {source}")]
    PeersNotAvailable { source: NetworkError },
    #[error("bootstrap pull request failed")]
    PullRequestFailed { source: NetworkError },
    #[error("bootstrap pull stream failed")]
    PullStreamFailed { source: NetworkError },
    #[error("decoding of a block failed")]
    BlockDecodingFailed {
        source: <Block as Deserialize>::Error,
    },
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

const MAX_BOOTSTRAP_PEERS: u32 = 32;

pub async fn peers_from_trusted_peer(peer: &Peer, logger: Logger) -> Result<Vec<Peer>, Error> {
    info!(
        logger,
        "getting peers from bootstrap peer {}", peer.connection
    );

    let client = grpc::connect(&peer, None)
        .await
        .map_err(|e| Error::Connect { source: e })?;
    let peers = client
        .peers(MAX_BOOTSTRAP_PEERS)
        .await
        .map_err(|e| Error::PeersNotAvailable { source: e })?;
    info!(
        logger,
        "peer {} : peers known : {}",
        peer.connection,
        peers.len()
    );
    let peers = peers.iter().map(|peer| Peer::new(peer.addr())).collect();
    Ok(peers)
}

pub async fn bootstrap_from_peer(
    peer: &Peer,
    blockchain: Blockchain,
    tip: Tip,
    logger: Logger,
) -> Result<(), Error> {
    debug!(logger, "connecting to bootstrap peer {}", peer.connection);

    let client = grpc::connect(&peer, None)
        .await
        .map_err(|e| Error::Connect { source: e })?;

    let checkpoints = blockchain.get_checkpoints(tip.branch()).await;
    let checkpoints = net_data::block::try_ids_from_iter(checkpoints).unwrap();

    info!(
        logger,
        "pulling blocks starting from checkpoints: {:?}", checkpoints
    );
    let stream = client
        .pull_blocks_to_tip(checkpoints)
        .await
        .map_err(|e| Error::PullRequestFailed { source: e })?;
    bootstrap_from_stream(blockchain, tip, stream, logger).await
}

struct BootstrapInfo {
    last_reported: std::time::SystemTime,
    last_bytes_received: u64,
    bytes_received: u64,
    block_received: u64,
    last_block_description: Option<HeaderDesc>,
}

impl BootstrapInfo {
    pub fn new() -> Self {
        let now = std::time::SystemTime::now();
        let lbd: Option<HeaderDesc> = None;
        BootstrapInfo {
            last_reported: now,
            last_bytes_received: 0,
            bytes_received: 0,
            block_received: 0,
            last_block_description: lbd,
        }
    }

    pub fn append_block(&mut self, b: &Block) {
        use chain_core::property::Serialize;
        self.bytes_received += b.serialize_as_vec().unwrap().len() as u64; // TODO sad serialization back
        self.block_received += 1;
        self.last_block_description = Some(b.header.description());
    }

    pub fn report(&mut self, logger: &Logger) {
        fn print_sz(n: f64) -> String {
            if n > 1_000_000.0 {
                format!("{:.2}mb", n / (1024 * 1024) as f64)
            } else if n > 1_000.0 {
                format!("{:.2}kb", n / 1024 as f64)
            } else {
                format!("{:.2}b", n)
            }
        }
        let current = std::time::SystemTime::now();
        let time_diff = current.duration_since(self.last_reported);
        let bytes_diff = self.bytes_received - self.last_bytes_received;

        let bytes = print_sz(bytes_diff as f64);
        let kbs = time_diff
            .map(|td| {
                let v = (bytes_diff as f64) / td.as_secs_f64();
                print_sz(v)
            })
            .unwrap_or("N/A".to_string());

        self.last_reported = current;
        self.last_bytes_received = self.bytes_received;
        info!(
            logger,
            "receiving from network bytes={} {}/s, blockchain {}",
            bytes,
            kbs,
            self.last_block_description
                .as_ref()
                .map(|lbd| lbd.to_string())
                .expect("append_block should always be called before report")
        )
    }
}

async fn bootstrap_from_stream<S>(
    blockchain: Blockchain,
    branch: Tip,
    stream: S,
    logger: Logger,
) -> Result<(), Error>
where
    S: Stream<Item = Result<net_data::Block, NetworkError>>,
{
    let block0 = blockchain.block0().clone();
    let logger2 = logger.clone();
    let blockchain2 = blockchain.clone();
    let branch2 = branch.clone();

    stream
        .map_err(|e| Error::PullStreamFailed { source: e })
        .and_then(|block| async {
            Block::deserialize(block.as_bytes())
                .map_err(|e| Error::BlockDecodingFailed { source: e })
        })
        .try_skip_while(|&block| async move { Ok(block.header.hash() == block0) })
        .map(|res| Ok(res))
        .try_fold(
            (BootstrapInfo::new(), None),
            move |(mut bi, parent_tip): (BootstrapInfo, Option<Arc<Ref>>), block_or_err| async {
                match block_or_err {
                    Ok(block) => {
                        const PROCESS_LOGGING_DISTANCE: u64 = 2500;
                        bi.append_block(&block);
                        if bi.block_received % PROCESS_LOGGING_DISTANCE == 0 {
                            bi.report(&logger)
                        }

                        handle_block(blockchain.clone(), block, logger.clone())
                            .await
                            .map(move |aref| (bi, Some(aref)))
                    }
                    Err(e) => {
                        if let Some(parent_tip) = parent_tip {
                            let _ = blockchain::process_new_ref_owned(
                                logger.clone(),
                                blockchain.clone(),
                                branch.clone(),
                                parent_tip.clone(),
                            )
                            .await;
                        }
                        Err(e)
                    }
                }
            },
        )
        .and_then(move |(_, maybe_new_tip)| async {
            if let Some(new_tip) = maybe_new_tip {
                blockchain::process_new_ref_owned(logger2, blockchain2, branch2, new_tip.clone())
                    .await
                    .map_err(|e| Error::ChainSelectionFailed { source: e })
            } else {
                info!(logger2, "no new blocks in bootstrap stream");
                Ok(())
            }
        })
        .await
}

async fn handle_block(
    blockchain: Blockchain,
    block: Block,
    logger: Logger,
) -> Result<Arc<Ref>, Error> {
    let header = block.header();
    let pre_checked = blockchain
        .pre_check_header(header, true)
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
                .post_check_header(header, parent_ref, blockchain::CheckHeaderProof::Enabled)
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
            Ok(applied.cached_ref())
        }
    }
}
