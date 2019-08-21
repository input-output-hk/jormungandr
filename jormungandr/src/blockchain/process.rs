use super::{Blockchain, Branch, CandidateRepo, Error, ErrorKind, PreCheckedHeader, Ref};
use crate::{
    blockcfg::{Block, Epoch, Header},
    intercom::{self, BlockMsg, NetworkMsg, PropagateMsg},
    leadership::NewEpochToSchedule,
    network::p2p::topology::NodeId,
    stats_counter::StatsCounter,
    utils::{
        async_msg::MessageBox,
        task::{Input, TokioServiceInfo},
    },
};
use chain_core::property::{Block as _, HasHeader as _};

use futures::future::Either;
use slog::Logger;
use tokio::{prelude::*, sync::mpsc::Sender};

pub fn handle_input(
    info: &TokioServiceInfo,
    blockchain: &mut Blockchain,
    blockchain_tip: &mut Branch,
    candidate_repo: &CandidateRepo,
    _stats_counter: &StatsCounter,
    new_epoch_announcements: &mut Sender<NewEpochToSchedule>,
    network_msg_box: &mut MessageBox<NetworkMsg>,
    input: Input<BlockMsg>,
) -> Result<(), ()> {
    let bquery = match input {
        Input::Shutdown => {
            // TODO: is there some work to do here to clean up the
            //       the state and make sure all state is saved properly
            return Ok(());
        }
        Input::Input(msg) => msg,
    };

    match bquery {
        BlockMsg::LeadershipExpectEndOfEpoch(epoch) => {
            handle_end_of_epoch(
                info.logger().new(o!()),
                new_epoch_announcements.clone(),
                blockchain.clone(),
                blockchain_tip.clone(),
                epoch + 1, // next epoch
            )
            .wait()
            .unwrap_or_else(|err| {
                crit!(
                    info.logger(),
                    "cannot send new leader schedule data to leadership module";
                    "reason" => err.to_string()
                )
            });
        }
        BlockMsg::LeadershipBlock(block) => {
            let future = process_leadership_block(info.logger(), blockchain.clone(), block);
            let new_block_ref = future.wait().unwrap();
            let header = new_block_ref.header().clone();
            blockchain_tip.update_ref(new_block_ref).wait().unwrap();
            network_msg_box
                .try_send(NetworkMsg::Propagate(PropagateMsg::Block(header)))
                .unwrap_or_else(|err| {
                    error!(info.logger(), "cannot propagate block to network: {}", err)
                });
        }
        BlockMsg::AnnouncedBlock(header, node_id) => {
            let future = process_block_announcement(
                blockchain.clone(),
                blockchain_tip.clone(),
                header,
                node_id,
                network_msg_box.clone(),
                info.logger().clone(),
            );
            future.wait().unwrap();
        }
        BlockMsg::NetworkBlock(block, reply) => {
            let future = process_network_block(blockchain.clone(), block, info.logger().clone());
            match future.wait() {
                Err(e) => {
                    reply.reply_error(network_block_error_into_reply(e));
                }
                Ok(maybe_updated) => {
                    if let Some(new_block_ref) = maybe_updated {
                        let header = new_block_ref.header().clone();
                        blockchain_tip.update_ref(new_block_ref).wait().unwrap();
                        network_msg_box
                            .try_send(NetworkMsg::Propagate(PropagateMsg::Block(header)))
                            .unwrap_or_else(|err| {
                                error!(info.logger(), "cannot propagate block to network: {}", err)
                            });
                    }
                    reply.reply_ok(());
                }
            }
        }
        BlockMsg::ChainHeaders(headers, reply) => {
            let future = candidate_repo.advance_branch(headers);
            match future.wait() {
                Err(e) => {
                    reply.reply_error(chain_header_error_into_reply(e));
                }
                Ok(hashes) => {
                    network_msg_box
                        .try_send(NetworkMsg::GetBlocks(hashes))
                        .unwrap_or_else(|err| {
                            error!(info.logger(), "cannot request blocks from network: {}", err)
                        });
                    reply.reply_ok(());
                }
            }
        }
    };

    Ok(())
}

pub fn handle_end_of_epoch(
    logger: Logger,
    new_epoch_announcements: Sender<NewEpochToSchedule>,
    mut blockchain: Blockchain,
    blockchain_tip: Branch,
    epoch: Epoch,
) -> impl Future<Item = (), Error = Error> {
    debug!(logger, "preparing new epoch schedule" ; "epoch" => epoch);
    blockchain_tip
        .get_ref()
        .map_err(|_: std::convert::Infallible| unreachable!())
        .and_then(move |ref_tip| {
            let (new_schedule, new_parameters, time_frame, _) =
                blockchain.new_epoch_leadership_from(epoch, ref_tip);

            new_epoch_announcements
                .send(NewEpochToSchedule {
                    new_schedule,
                    new_parameters,
                    time_frame: (*time_frame).clone(),
                })
                .map_err(move |_err| {
                    crit!(
                        logger,
                        "cannot send new epoch schedule data to leadership module"
                    );
                    "unable to process new epoch schedule".into()
                })
        })
        .map(|_| ())
}

pub fn process_leadership_block(
    logger: &Logger,
    mut blockchain: Blockchain,
    block: Block,
) -> impl Future<Item = Ref, Error = Error> {
    let mut end_blockchain = blockchain.clone();
    let header = block.header();
    let parent_hash = block.parent_id();

    let logger = logger.new(o!(
        "hash" => header.hash().to_string(),
        "parent" => parent_hash.to_string(),
        "date" => header.block_date().to_string()));
    let logger1 = logger.clone();
    // This is a trusted block from the leadership task,
    // so we can skip pre-validation.
    blockchain
        .get_ref(parent_hash)
        .and_then(move |parent| {
            if let Some(parent_ref) = parent {
                debug!(logger1, "processing block from leader event");
                Either::A(blockchain.post_check_header(header, parent_ref))
            } else {
                error!(
                    logger1,
                    "block from leader event does not have parent block in storage"
                );
                Either::B(future::err(
                    ErrorKind::MissingParentBlockFromStorage(header).into(),
                ))
            }
        })
        .and_then(move |post_checked| end_blockchain.apply_and_store_block(post_checked, block))
        .map(move |e| {
            info!(logger, "block from leader event successfully stored");
            e
        })
}

pub fn process_block_announcement(
    mut blockchain: Blockchain,
    branch: Branch,
    header: Header,
    node_id: NodeId,
    mut network_msg_box: MessageBox<NetworkMsg>,
    logger: Logger,
) -> impl Future<Item = (), Error = Error> {
    blockchain
        .pre_check_header(header)
        .and_then(move |pre_checked| match pre_checked {
            PreCheckedHeader::AlreadyPresent { .. } => {
                debug!(logger, "block is already present");
                Either::A(future::ok(()))
            }
            PreCheckedHeader::MissingParent { header, .. } => {
                debug!(logger, "block is missing a locally stored parent");
                let to = header.hash();
                Either::B(blockchain.get_checkpoints(branch).map(move |from| {
                    network_msg_box
                        .try_send(NetworkMsg::PullHeaders { node_id, from, to })
                        .unwrap_or_else(move |err| {
                            error!(
                                logger,
                                "cannot send PullHeaders request to network: {}", err
                            )
                        });
                }))
            }
            PreCheckedHeader::HeaderWithCache { header, .. } => {
                debug!(
                    logger,
                    "Announced block has a locally stored parent, fetch it"
                );
                network_msg_box
                    .try_send(NetworkMsg::GetNextBlock(node_id, header.hash()))
                    .unwrap_or_else(move |err| {
                        error!(
                            logger,
                            "cannot send GetNextBlock request to network: {}", err
                        )
                    });
                Either::A(future::ok(()))
            }
        })
}

pub fn process_network_block(
    mut blockchain: Blockchain,
    block: Block,
    logger: Logger,
) -> impl Future<Item = Option<Ref>, Error = Error> {
    let mut end_blockchain = blockchain.clone();
    let header = block.header();
    blockchain
        .pre_check_header(header)
        .and_then(move |pre_checked| match pre_checked {
            PreCheckedHeader::AlreadyPresent { .. } => {
                debug!(logger, "block is already present");
                Either::A(future::ok(None))
            }
            PreCheckedHeader::MissingParent { header, .. } => {
                debug!(logger, "block is missing a locally stored parent");
                Either::A(future::err(
                    ErrorKind::BlockHeaderMissingParent(header.block_parent_hash().clone()).into(),
                ))
            }
            PreCheckedHeader::HeaderWithCache { header, parent_ref } => {
                let post_check_and_apply = blockchain
                    .post_check_header(header, parent_ref)
                    .and_then(move |post_checked| {
                        end_blockchain.apply_and_store_block(post_checked, block)
                    })
                    .map(move |block_ref| {
                        debug!(logger, "block successfully applied");
                        Some(block_ref)
                    });
                Either::B(post_check_and_apply)
            }
        })
}

fn network_block_error_into_reply(err: Error) -> intercom::Error {
    use super::ErrorKind::*;

    match err.0 {
        Storage(e) => intercom::Error::failed(e),
        Ledger(e) => intercom::Error::failed_precondition(e),
        Block0(e) => intercom::Error::failed(e),
        MissingParentBlockFromStorage(_) => intercom::Error::failed_precondition(err.to_string()),
        BlockHeaderMissingParent(_) => intercom::Error::invalid_argument(err.to_string()),
        BlockHeaderVerificationFailed(_) => intercom::Error::invalid_argument(err.to_string()),
        _ => intercom::Error::failed(err.to_string()),
    }
}

fn chain_header_error_into_reply(err: Error) -> intercom::Error {
    use super::ErrorKind::*;

    // TODO: more detailed error case matching
    match err.0 {
        Storage(e) => intercom::Error::failed(e),
        Ledger(e) => intercom::Error::failed_precondition(e),
        Block0(e) => intercom::Error::failed(e),
        MissingParentBlockFromStorage(_) => intercom::Error::failed_precondition(err.to_string()),
        BlockHeaderVerificationFailed(_) => intercom::Error::invalid_argument(err.to_string()),
        _ => intercom::Error::failed(err.to_string()),
    }
}
