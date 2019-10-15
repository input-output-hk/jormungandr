use super::{grpc, BlockConfig};
use crate::blockcfg::{Block, HeaderHash};
use crate::blockchain::{
    self, Blockchain, Error as BlockchainError, PreCheckedHeader, Ref, Tip, MAIN_BRANCH_TAG,
};
use crate::settings::start::network::Peer;
use chain_core::property::HasHeader;
use chain_storage::error::Error as StorageError;
use network_core::client::{BlockService, Client as _};
use network_core::error::Error as NetworkError;
use network_grpc::client::Connection;
use slog::Logger;
use tokio::prelude::*;
use tokio::runtime::current_thread;

use std::error;
use std::fmt::{self, Debug, Display};
use std::sync::Arc;

type ConnectError = network_grpc::client::ConnectError<std::io::Error>;

#[derive(Debug)]
pub enum Error {
    Connect(ConnectError),
    ClientNotReady(NetworkError),
    PullRequestFailed(NetworkError),
    PullStreamFailed(NetworkError),
    HeaderCheckFailed(BlockchainError),
    BlockAlreadyPresent(HeaderHash),
    BlockMissingParent(HeaderHash),
    ApplyBlockFailed(BlockchainError),
    ChainSelectionFailed(blockchain::ProcessError),
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::Error::*;
        match self {
            Connect(_) => write!(f, "failed to connect to bootstrap peer"),
            ClientNotReady(_) => write!(f, "connection broken"),
            PullRequestFailed(_) => write!(f, "bootstrap pull request failed"),
            PullStreamFailed(_) => write!(f, "bootstrap pull stream failed"),
            HeaderCheckFailed(_) => write!(f, "block header check failed"),
            BlockAlreadyPresent(hash) => write!(f, "received block {} is already present", hash),
            BlockMissingParent(hash) => write!(
                f,
                "received block {} is not connected to the block chain",
                hash
            ),
            ApplyBlockFailed(_) => write!(f, "failed to apply block to the blockchain"),
            ChainSelectionFailed(_) => write!(f, "failed to select the new tip"),
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        use self::Error::*;
        match self {
            Connect(e) => Some(e),
            ClientNotReady(e) => Some(e),
            PullRequestFailed(e) => Some(e),
            PullStreamFailed(e) => Some(e),
            HeaderCheckFailed(e) => Some(e),
            BlockAlreadyPresent(_) => None,
            BlockMissingParent(_) => None,
            ApplyBlockFailed(e) => Some(e),
            ChainSelectionFailed(e) => Some(e),
        }
    }
}

pub fn bootstrap_from_peer(
    peer: Peer,
    blockchain: Blockchain,
    branch: Tip,
    logger: &Logger,
) -> Result<Arc<Ref>, Error> {
    info!(logger, "connecting to bootstrap peer {}", peer.connection);

    let blockchain2 = blockchain.clone();

    let bootstrap = grpc::connect(peer.address(), None)
        .map_err(Error::Connect)
        .and_then(|client: Connection<BlockConfig>| client.ready().map_err(Error::ClientNotReady))
        .join(branch.get_ref().map_err(|_| unreachable!()))
        .and_then(|(mut client, tip)| {
            let tip_hash = tip.hash();
            debug!(logger, "pulling blocks starting from {}", tip_hash);
            client
                .pull_blocks_to_tip(&[tip_hash])
                .map_err(Error::PullRequestFailed)
                .and_then(|stream| bootstrap_from_stream(blockchain, tip, stream, logger.clone()))
        })
        .and_then(move |tip| {
            blockchain::process_new_ref(logger.clone(), blockchain2, branch, tip.clone())
                .map_err(Error::ChainSelectionFailed)
                .map(|()| tip)
        });

    current_thread::block_on_all(bootstrap)
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
        .map_err(Error::PullStreamFailed)
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
        .map_err(Error::HeaderCheckFailed)
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
                .map_err(Error::HeaderCheckFailed)
        })
        .and_then(move |post_checked| {
            end_blockchain
                .apply_and_store_block(post_checked, block)
                .map_err(Error::ApplyBlockFailed)
        })
}
