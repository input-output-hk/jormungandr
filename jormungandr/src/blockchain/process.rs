use super::{
    compare_against, Blockchain, ComparisonResult, Error, ErrorKind, PreCheckedHeader, Ref, Tip,
    MAIN_BRANCH_TAG,
};
use crate::{
    blockcfg::{Block, Epoch, Header, HeaderHash},
    intercom::{self, BlockMsg, ExplorerMsg, NetworkMsg, PropagateMsg},
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

use std::{convert::identity, sync::Arc};

pub fn handle_input(
    info: &TokioServiceInfo,
    blockchain: &mut Blockchain,
    blockchain_tip: &mut Tip,
    stats_counter: &StatsCounter,
    new_epoch_announcements: &mut Sender<NewEpochToSchedule>,
    network_msg_box: &mut MessageBox<NetworkMsg>,
    explorer_msg_box: &mut Option<MessageBox<ExplorerMsg>>,
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
            process_new_ref(
                blockchain.clone(),
                blockchain_tip.clone(),
                new_block_ref.clone(),
            )
            .wait()
            .unwrap();
            network_msg_box
                .try_send(NetworkMsg::Propagate(PropagateMsg::Block(header)))
                .unwrap_or_else(|err| {
                    error!(info.logger(), "cannot propagate block to network: {}", err)
                });

            if let Some(msg_box) = explorer_msg_box {
                /*
                msg_box
                    .try_send(ExplorerMsg::NewBlock(new_block_ref))
                    .unwrap_or_else(|err| {
                        error!(info.logger(), "cannot add block to explorer: {}", err)
                    });
                */
            };
            stats_counter.add_block_recv_cnt(1);
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
            stats_counter.add_block_recv_cnt(1);
            let future = process_network_block(blockchain.clone(), block, info.logger().clone());
            match future.wait() {
                Err(e) => {
                    reply.reply_error(network_block_error_into_reply(e));
                }
                Ok(maybe_updated) => {
                    if let Some(new_block_ref) = maybe_updated {
                        let header = new_block_ref.header().clone();
                        process_new_ref(
                            blockchain.clone(),
                            blockchain_tip.clone(),
                            new_block_ref.clone(),
                        )
                        .wait()
                        .unwrap();
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
        BlockMsg::ChainHeaders(stream) => unimplemented!(),
    };

    Ok(())
}

/// process a new candidate block on top of the blockchain, this function may:
///
/// * update the current tip if the candidate's parent is the current tip;
/// * update a branch if the candidate parent is that branch's tip;
/// * create a new branch if none of the above;
///
/// If the current tip is not the one being updated we will then trigger
/// chain selection after updating that other branch as it may be possible that
/// this branch just became more interesting for the current consensus algorithm.
pub fn process_new_ref(
    mut blockchain: Blockchain,
    mut tip: Tip,
    candidate: Arc<Ref>,
) -> impl Future<Item = (), Error = Error> {
    use tokio::prelude::future::Either::*;

    let candidate_hash = candidate.hash();
    let mut storage = blockchain.storage().clone();

    tip.clone()
        .get_ref()
        .and_then(move |tip_ref| {
            if &tip_ref.hash() == candidate.block_parent_hash() {
                A(A(tip.update_ref(candidate).map(|_| true)))
            } else {
                match compare_against(blockchain.storage(), &tip_ref, &candidate) {
                    ComparisonResult::PreferCurrent => A(B(future::ok(false))),
                    ComparisonResult::PreferCandidate => B(blockchain
                        .branches_mut()
                        .apply_or_create(candidate)
                        .and_then(move |branch| tip.swap(branch))
                        .map(|()| true)),
                }
            }
        })
        .map_err(|_: std::convert::Infallible| unreachable!())
        .and_then(move |tip_updated| {
            if tip_updated {
                A(storage
                    .put_tag(MAIN_BRANCH_TAG.to_owned(), candidate_hash)
                    .map_err(|e| Error::with_chain(e, "Cannot update the main storage's tip")))
            } else {
                B(future::ok(()))
            }
        })
}

pub fn handle_end_of_epoch(
    logger: Logger,
    new_epoch_announcements: Sender<NewEpochToSchedule>,
    mut blockchain: Blockchain,
    blockchain_tip: Tip,
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
) -> impl Future<Item = Arc<Ref>, Error = Error> {
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
    blockchain_tip: Tip,
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
                Either::B(
                    blockchain
                        .get_checkpoints(blockchain_tip.branch().clone())
                        .map(move |from| {
                            network_msg_box
                                .try_send(NetworkMsg::PullHeaders { node_id, from, to })
                                .unwrap_or_else(move |err| {
                                    error!(
                                        logger,
                                        "cannot send PullHeaders request to network: {}", err
                                    )
                                });
                        }),
                )
            }
            PreCheckedHeader::HeaderWithCache { header, parent_ref } => {
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
) -> impl Future<Item = Option<Arc<Ref>>, Error = Error> {
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
                    ErrorKind::MissingParentBlockFromStorage(header).into(),
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
        BlockHeaderVerificationFailed(_) => intercom::Error::invalid_argument(err.to_string()),
        _ => intercom::Error::failed(err.to_string()),
    }
}

pub fn process_chain_headers_into_block_request<S>(
    mut blockchain: Blockchain,
    headers: S,
    logger: Logger,
) -> impl Future<Item = Vec<HeaderHash>, Error = Error>
where
    S: Stream<Item = Header>,
{
    headers
        .map_err(|e| {
            // TODO: map the incoming stream error to the result error
            unimplemented!()
        })
        .and_then(move |header| {
            blockchain
                .pre_check_header(header)
                .and_then(move |pre_checked| match pre_checked {
                    PreCheckedHeader::AlreadyPresent { .. } => {
                        // The block is already present. This may happen
                        // if the peer has started from an earlier checkpoint
                        // than our tip, so ignore this and proceed.
                        Ok(None)
                    }
                    PreCheckedHeader::MissingParent { header, .. } => {
                        // TODO: this fails on the first header after the
                        // immediate descendant of the local tip. Need branch storage
                        // that would store the whole header chain without blocks,
                        // so that the chain can be pre-validated first and blocks
                        // fetched afterwards in arbitrary order.
                        Err(ErrorKind::MissingParentBlockFromStorage(header).into())
                    }
                    PreCheckedHeader::HeaderWithCache { header, parent_ref } => {
                        // TODO: limit the headers to the single epoch
                        // before pausing to retrieve blocks.
                        Ok(Some(header.hash()))
                    }
                })
        })
        .filter_map(identity)
        .collect()
}
