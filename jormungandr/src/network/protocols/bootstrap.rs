use super::super::{grpc, BlockConfig};
use crate::blockcfg::Block;
use crate::blockchain::protocols::{Blockchain, PreCheckedHeader, Ref};
use crate::settings::start::network::Peer;
use chain_core::property::HasHeader;
use network_core::client::{block::BlockService, Client as _};
use network_grpc::client::Connection;
use slog::Logger;
use std::fmt::Debug;
use tokio::prelude::*;
use tokio::runtime::current_thread;

pub fn bootstrap_from_peer(
    peer: Peer,
    blockchain: Blockchain,
    tip: Ref,
    logger: &Logger,
) -> Result<Ref, ()> {
    info!(logger, "connecting to bootstrap peer {}", peer.connection);
    let bootstrap = grpc::connect(peer.address(), None)
        .map_err(|e| {
            error!(logger, "failed to connect to bootstrap peer: {:?}", e);
        })
        .and_then(|client: Connection<BlockConfig>| {
            client.ready().map_err(|e| {
                error!(logger, "bootstrap peer disconnected: {:?}", e);
            })
        })
        .and_then(|mut client| {
            let tip_hash = *tip.hash();
            client
                .pull_blocks_to_tip(&[tip_hash])
                .map_err(|e| {
                    error!(logger, "PullBlocksToTip request failed: {:?}", e);
                })
                .and_then(|stream| bootstrap_from_stream(blockchain, tip, stream, logger.clone()))
        });

    current_thread::block_on_all(bootstrap).map(|tip| {
        debug!(logger, "bootstrap complete");
        tip
    })
}

fn bootstrap_from_stream<S>(
    blockchain: Blockchain,
    tip: Ref,
    stream: S,
    logger: Logger,
) -> impl Future<Item = Ref, Error = ()>
where
    S: Stream<Item = Block>,
    S::Error: Debug,
{
    let fold_logger = logger.clone();
    stream
        .map_err(move |e| {
            error!(logger, "bootstrap block streaming failed: {:?}", e);
        })
        .fold(tip, move |tip, block| {
            handle_block(blockchain.clone(), block, fold_logger.clone())
        })
}

fn handle_block(
    mut blockchain: Blockchain,
    block: Block,
    logger: Logger,
) -> impl Future<Item = Ref, Error = ()> {
    let header = block.header();
    debug!(
        logger,
        "received block from the bootstrap node: {:#?}", header
    );
    let err1_logger = logger.clone();
    let err2_logger = logger.clone();
    let err3_logger = logger.clone();
    let mut end_blockchain = blockchain.clone();
    blockchain
        .pre_check_header(header)
        .map_err(move |e| {
            warn!(err1_logger, "header pre-check failed: {:?}", e);
        })
        .and_then(move |pre_checked| match pre_checked {
            PreCheckedHeader::AlreadyPresent { header, .. } => {
                warn!(logger, "block {} is already present", header.hash());
                Err(())
            }
            PreCheckedHeader::MissingParent { header, .. } => {
                warn!(logger, "received a disconnected block {}", header.hash());
                Err(())
            }
            PreCheckedHeader::HeaderWithCache { header, parent_ref } => Ok((header, parent_ref)),
        })
        .and_then(move |(header, parent_ref)| {
            blockchain
                .post_check_header(header, parent_ref)
                .map_err(move |e| {
                    warn!(err2_logger, "header post-check failed: {:?}", e);
                })
        })
        .and_then(move |post_checked| {
            end_blockchain
                .apply_block(post_checked, block)
                .map_err(move |e| {
                    error!(err3_logger, "failed to apply block to storage: {:?}", e);
                })
        })
}
