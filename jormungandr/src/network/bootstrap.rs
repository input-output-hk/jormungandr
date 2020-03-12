use super::{grpc, BlockConfig};
use crate::blockcfg::{Block, HeaderDesc, HeaderHash};
use crate::blockchain::{self, Blockchain, Error as BlockchainError, PreCheckedHeader, Ref, Tip};
use crate::settings::start::network::Peer;
use chain_core::property::HasHeader;
use chain_network::error::Error as NetworkError;
use slog::Logger;
use thiserror::Error;
use tokio02::runtime;

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
    #[error("peers not available {source}")]
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

    let mut runtime = runtime::Builder::new()
        .thread_name("peer-resolver")
        .core_threads(2)
        .build()
        .map_err(|e| Error::RuntimeInit { source: e })?;
    let bootstrap = grpc::connect(&peer, None, runtime.executor())
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
                        .map(|peer| Peer::new(peer.addr))
                        .collect();
                    future::ok(peers)
                })
        });

    runtime.block_on(bootstrap)
}

pub fn bootstrap_from_peer(
    peer: &Peer,
    blockchain: Blockchain,
    tip: Tip,
    logger: Logger,
) -> Result<(), Error> {
    use futures03::future::try_join;

    debug!(logger, "connecting to bootstrap peer {}", peer.connection);

    let mut runtime = Runtime::new().map_err(|e| Error::RuntimeInit { source: e })?;
    let grpc_executor = runtime.executor();

    runtime.block_on_std(async move {
        let client = grpc::connect(&peer, None, grpc_executor)
            .compat()
            .await
            .map_err(|e| Error::Connect { source: e })?;

        let (mut client, checkpoints) = try_join(
            async {
                client
                    .ready()
                    .compat()
                    .await
                    .map_err(|e| Error::ClientNotReady { source: e })
            },
            async { Ok(blockchain.get_checkpoints(tip.branch()).await) },
        )
        .await?;

        info!(
            logger,
            "pulling blocks starting from checkpoints: {:?}", checkpoints
        );
        let stream = client
            .pull_blocks_to_tip(checkpoints.as_slice())
            .compat()
            .await
            .map_err(|e| Error::PullRequestFailed { source: e })?;
        bootstrap_from_stream(blockchain, tip, stream, logger)
            .compat()
            .await
    })
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
            (BootstrapInfo::new(), None),
            move |(mut bi, parent_tip): (BootstrapInfo, Option<Arc<Ref>>), block_or_err| {
                match block_or_err {
                    Ok(block) => {
                        const PROCESS_LOGGING_DISTANCE: u64 = 2500;
                        bi.append_block(&block);
                        if bi.block_received % PROCESS_LOGGING_DISTANCE == 0 {
                            bi.report(&logger)
                        }

                        let fut = handle_block_old(blockchain.clone(), block, logger.clone())
                            .map(move |aref| (bi, Some(aref)));
                        Either::A(fut)
                    }
                    Err(e) => {
                        let fut = if let Some(parent_tip) = parent_tip {
                            Either::A(process_new_ref_old(
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
                }
            },
        )
        .and_then(move |(_, maybe_new_tip)| {
            if let Some(new_tip) = maybe_new_tip {
                Either::A(
                    process_new_ref_old(logger2, blockchain2, branch2, new_tip.clone())
                        .map_err(|e| Error::ChainSelectionFailed { source: e }),
                )
            } else {
                info!(logger2, "no new blocks in bootstrap stream");
                Either::B(future::ok(()))
            }
        })
}

fn process_new_ref_old(
    logger: Logger,
    blockchain: Blockchain,
    tip: Tip,
    candidate: Arc<Ref>,
) -> impl Future<Item = (), Error = blockchain::Error> {
    Compat::new(Box::pin(blockchain::process_new_ref_owned(
        logger, blockchain, tip, candidate,
    )))
}

fn handle_block_old(
    blockchain: Blockchain,
    block: Block,
    logger: Logger,
) -> impl Future<Item = Arc<Ref>, Error = Error> {
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
