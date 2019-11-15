use super::{
    candidate::{self, CandidateForest},
    chain,
    chain_selection::{self, ComparisonResult},
    Blockchain, Error, ErrorKind, PreCheckedHeader, Ref, Tip, MAIN_BRANCH_TAG,
};
use crate::{
    blockcfg::{Block, FragmentId, Header},
    intercom::{self, BlockMsg, ExplorerMsg, NetworkMsg, PropagateMsg, TransactionMsg},
    network::p2p::Id as NodeId,
    stats_counter::StatsCounter,
    utils::{
        async_msg::{self, MessageBox},
        task::{Input, TokioServiceInfo},
    },
};
use chain_core::property::{Block as _, Fragment as _, HasHeader as _, Header as _};
use jormungandr_lib::interfaces::FragmentStatus;

use futures::future::Either;
use slog::Logger;
use tokio::prelude::*;

use std::sync::Arc;

pub fn handle_input(
    info: &TokioServiceInfo,
    blockchain: &mut Blockchain,
    blockchain_tip: &mut Tip,
    candidate_forest: &CandidateForest,
    stats_counter: &StatsCounter,
    network_msg_box: &mut MessageBox<NetworkMsg>,
    tx_msg_box: &mut MessageBox<TransactionMsg>,
    explorer_msg_box: Option<&mut MessageBox<ExplorerMsg>>,
    input: Input<BlockMsg>,
) -> impl Future<Item = (), Error = ()> {
    match input {
        Input::Shutdown => {
            // TODO: is there some work to do here to clean up the
            //       the state and make sure all state is saved properly
            Either::A(future::ok(()))
        }
        Input::Input(msg) => {
            let logger = info.logger().clone();
            Either::B(
                run_handle_input(
                    info,
                    blockchain.clone(),
                    blockchain_tip.clone(),
                    candidate_forest.clone(),
                    stats_counter.clone(),
                    network_msg_box.clone(),
                    tx_msg_box.clone(),
                    explorer_msg_box.cloned(),
                    msg,
                )
                .map_err(move |e| {
                    error!(
                        logger,
                        "Cannot process block event" ;
                        "reason" => %e,
                    );
                }),
            )
        }
    }
}

pub fn run_handle_input(
    info: &TokioServiceInfo,
    blockchain: Blockchain,
    blockchain_tip: Tip,
    candidate_forest: CandidateForest,
    stats_counter: StatsCounter,
    network_msg_box: MessageBox<NetworkMsg>,
    mut tx_msg_box: MessageBox<TransactionMsg>,
    explorer_msg_box: Option<MessageBox<ExplorerMsg>>,
    input: BlockMsg,
) -> impl Future<Item = (), Error = Error> {
    match input {
        BlockMsg::LeadershipBlock(block) => {
            let logger = info.logger().new(o!(
                "hash" => block.header.hash().to_string(),
                "parent" => block.header.parent_id().to_string(),
                "date" => block.header.block_date().to_string()));

            let process_new_block =
                process_leadership_block(logger.clone(), blockchain.clone(), block.clone());

            let fragments = block.fragments().map(|f| f.id()).collect();

            let update_mempool = process_new_block.and_then(move |new_block_ref| {
                try_request_fragment_removal(&mut tx_msg_box, fragments, new_block_ref.header())
                    .map_err(|_| "cannot remove fragments from pool".into())
                    .map(|_| {
                        stats_counter.add_block_recv_cnt(1);
                        new_block_ref
                    })
            });

            let process_new_ref = update_mempool.and_then(move |new_block_ref| {
                process_and_propagate_new_ref(
                    logger,
                    blockchain,
                    blockchain_tip,
                    Arc::clone(&new_block_ref),
                    network_msg_box,
                )
            });

            let notify_explorer = process_new_ref.and_then(move |()| {
                if let Some(msg_box) = explorer_msg_box {
                    Either::A(
                        msg_box
                            .send(ExplorerMsg::NewBlock(block))
                            .map_err(|_| "Cannot propagate block to explorer".into())
                            .map(|_| ()),
                    )
                } else {
                    Either::B(future::ok(()))
                }
            });

            Either::A(Either::A(notify_explorer))
        }
        BlockMsg::AnnouncedBlock(header, node_id) => {
            let logger = info.logger().new(o!(
                "hash" => header.hash().to_string(),
                "parent" => header.parent_id().to_string(),
                "date" => header.block_date().to_string(),
                "from_node_id" => node_id.to_string()));

            let future = process_block_announcement(
                blockchain.clone(),
                blockchain_tip.clone(),
                header,
                node_id,
                network_msg_box.clone(),
                logger,
            );

            Either::A(Either::B(future))
        }
        BlockMsg::NetworkBlocks(handle) => {
            let logger = info.logger().clone();
            let logger_fold = logger.clone();
            let blockchain_fold = blockchain.clone();
            let (stream, reply) = handle.into_stream_and_reply();
            let future = stream
                .map_err(|()| Error::from("Error while processing block input stream"))
                .fold(
                    (reply, stats_counter, None),
                    move |(reply, stats_counter, _), block| {
                        process_network_block(
                            blockchain_fold.clone(),
                            candidate_forest.clone(),
                            block,
                            tx_msg_box.clone(),
                            explorer_msg_box.clone(),
                            logger_fold.clone(),
                        )
                        .then(move |res| match res {
                            Ok(maybe_updated) => {
                                stats_counter.add_block_recv_cnt(1);
                                Ok((reply, stats_counter, maybe_updated))
                            }
                            Err(e) => {
                                reply.reply_error(network_block_error_into_reply(e));
                                Err(Error::from("Cannot propagate to block error"))
                            }
                        })
                    },
                )
                .and_then(move |(reply, _, maybe_updated)| {
                    if let Some(new_block_ref) = maybe_updated {
                        let future = process_and_propagate_new_ref(
                            logger,
                            blockchain,
                            blockchain_tip,
                            Arc::clone(&new_block_ref),
                            network_msg_box,
                        )
                        .map(|()| reply);
                        Either::A(future)
                    } else {
                        Either::B(future::ok(reply))
                    }
                })
                .map(|reply| reply.reply_ok(()));

            Either::B(Either::A(future))
        }
        BlockMsg::ChainHeaders(handle) => {
            let (stream, reply) = handle.into_stream_and_reply();
            let future = candidate_forest.advance_branch(stream);

            let future = future.then(|resp| match resp {
                Err(e) => {
                    reply.reply_error(chain_header_error_into_reply(e));
                    Either::A(future::err::<(), Error>(
                        format!("Error processing ChainHeader handling").into(),
                    ))
                }
                Ok((hashes, maybe_remainder)) => {
                    Either::B(
                        network_msg_box
                            .send(NetworkMsg::GetBlocks(hashes))
                            .map_err(|_| "cannot request blocks from network".into())
                            .map(|_| reply.reply_ok(())),
                    )
                    // TODO: if the stream is not ended, resume processing
                    // after more blocks arrive
                }
            });

            Either::B(Either::B(future))
        }
    }
}

fn try_request_fragment_removal(
    tx_msg_box: &mut MessageBox<TransactionMsg>,
    fragment_ids: Vec<FragmentId>,
    header: &Header,
) -> Result<(), async_msg::TrySendError<TransactionMsg>> {
    let hash = header.hash().into();
    let date = header.block_date().clone().into();
    let status = FragmentStatus::InABlock { date, block: hash };
    tx_msg_box.try_send(TransactionMsg::RemoveTransactions(fragment_ids, status))
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
    logger: Logger,
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
            if tip_ref.hash() == candidate.block_parent_hash() {
                info!(logger, "update current branch tip");
                A(A(tip.update_ref(candidate).map(|_| true)))
            } else {
                match chain_selection::compare_against(blockchain.storage(), &tip_ref, &candidate) {
                    ComparisonResult::PreferCurrent => {
                        info!(logger, "create new branch");
                        A(B(future::ok(false)))
                    }
                    ComparisonResult::PreferCandidate => {
                        info!(logger, "switching to new candidate branch");
                        B(blockchain
                            .branches_mut()
                            .apply_or_create(candidate)
                            .and_then(move |branch| tip.swap(branch))
                            .map(|()| true))
                    }
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

fn process_and_propagate_new_ref(
    logger: Logger,
    blockchain: Blockchain,
    tip: Tip,
    new_block_ref: Arc<Ref>,
    network_msg_box: MessageBox<NetworkMsg>,
) -> impl Future<Item = (), Error = Error> {
    let process_new_ref = process_new_ref(logger, blockchain, tip, new_block_ref.clone());

    process_new_ref.and_then(move |()| {
        let header = new_block_ref.header().clone();
        network_msg_box
            .send(NetworkMsg::Propagate(PropagateMsg::Block(header)))
            .map_err(|_| "Cannot propagate block to network".into())
            .map(|_| ())
    })
}

pub fn process_leadership_block(
    logger: Logger,
    mut blockchain: Blockchain,
    block: Block,
) -> impl Future<Item = Arc<Ref>, Error = Error> {
    let mut end_blockchain = blockchain.clone();
    let header = block.header();
    let parent_hash = block.parent_id();
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
                    ErrorKind::MissingParentBlock(parent_hash).into(),
                ))
            }
        })
        .and_then(move |post_checked| end_blockchain.apply_and_store_block(post_checked, block))
        .map_err(|err| Error::with_chain(err, "cannot process leadership block"))
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
        .pre_check_header(header, false)
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
            PreCheckedHeader::HeaderWithCache {
                header,
                parent_ref: _,
            } => {
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
        .map_err(|err| Error::with_chain(err, "cannot process block announcement"))
}

pub fn process_network_block(
    mut blockchain: Blockchain,
    candidate_forest: CandidateForest,
    block: Block,
    mut tx_msg_box: MessageBox<TransactionMsg>,
    mut explorer_msg_box: Option<MessageBox<ExplorerMsg>>,
    logger: Logger,
) -> impl Future<Item = Option<Arc<Ref>>, Error = chain::Error> {
    use futures::future::Either::{A, B};

    let logger = logger.new(o!(
        "hash" => block.header.hash().to_string(),
        "parent" => block.header.parent_id().to_string(),
        "date" => block.header.block_date().to_string()
    ));
    let end_logger = logger.clone();
    let mut end_blockchain = blockchain.clone();
    let explorer_enabled = explorer_msg_box.is_some();
    let header = block.header();
    blockchain
        .pre_check_header(header, false)
        .and_then(move |pre_checked| match pre_checked {
            PreCheckedHeader::AlreadyPresent { .. } => {
                debug!(logger, "block is already present");
                A(A(future::ok(None)))
            }
            PreCheckedHeader::MissingParent { .. } => {
                debug!(
                    logger,
                    "block is missing a locally stored parent, caching as candidate"
                );
                A(B(candidate_forest.cache_block(block).map(|()| None)))
            }
            PreCheckedHeader::HeaderWithCache { header, parent_ref } => {
                let post_check_and_apply = blockchain
                    .post_check_header(header, parent_ref)
                    .and_then(move |post_checked| {
                        let mut block_for_explorer = if explorer_enabled {
                            Some(block.clone())
                        } else {
                            None
                        };
                        let fragment_ids = block.fragments().map(|f| f.id()).collect::<Vec<_>>();
                        end_blockchain
                            .apply_and_store_block(post_checked, block)
                            .and_then(move |block_ref| {
                                try_request_fragment_removal(&mut tx_msg_box, fragment_ids, block_ref.header()).unwrap_or_else(|err| {
                                    error!(logger, "cannot remove fragments from pool" ; "reason" => %err)
                                });
                                if let Some(msg_box) = explorer_msg_box.as_mut() {
                                    msg_box
                                        .try_send(ExplorerMsg::NewBlock(block_for_explorer.take().unwrap()))
                                        .unwrap_or_else(|err| {
                                            error!(logger, "cannot add block to explorer: {}", err)
                                        });
                                }
                                Ok(block_ref)
                            })
                    })
                    .and_then(move |block_ref| {
                        candidate_forest
                            .on_applied_block(block_ref.hash())
                            .map_err(|never| match never {})
                            .map(|more_blocks| (block_ref, more_blocks))
                    })
                    .map(move |(block_ref, more_blocks)| {
                        info!(end_logger, "block successfully applied");
                        if !more_blocks.is_empty() {
                            warn!(
                                end_logger,
                                "{} more blocks have arrived out of order, \
                                 but I don't know what to do with them yet!",
                                more_blocks.len(),
                            );
                        }
                        Some(block_ref)
                    });
                B(post_check_and_apply)
            }
        })
}

fn network_block_error_into_reply(err: chain::Error) -> intercom::Error {
    use super::chain::ErrorKind::*;

    match err.0 {
        Storage(e) => intercom::Error::failed(e),
        Ledger(e) => intercom::Error::failed_precondition(e),
        Block0(e) => intercom::Error::failed(e),
        MissingParentBlock(_) => intercom::Error::failed_precondition(err.to_string()),
        BlockHeaderVerificationFailed(_) => intercom::Error::invalid_argument(err.to_string()),
        _ => intercom::Error::failed(err.to_string()),
    }
}

fn chain_header_error_into_reply(err: candidate::Error) -> intercom::Error {
    use super::candidate::Error::*;

    // TODO: more detailed error case matching
    match err {
        Storage(e) => intercom::Error::failed(e),
        EmptyHeaderStream => intercom::Error::invalid_argument(err),
        MissingParentBlock(_) => intercom::Error::failed_precondition(err),
        BrokenHeaderChain(_) => intercom::Error::invalid_argument(err),
        HeaderChainVerificationFailed(e) => intercom::Error::invalid_argument(e),
        _ => intercom::Error::failed(err),
    }
}
