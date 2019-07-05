use crate::blockcfg::{Header, HeaderHash};
use crate::blockchain::chain::{self, BlockHeaderTriage, BlockchainR, HandledBlock};
use crate::intercom::{self, BlockMsg, NetworkMsg, PropagateMsg};
use crate::stats_counter::StatsCounter;
use crate::utils::{
    async_msg::MessageBox,
    task::{Input, TokioServiceInfo},
};

use chain_core::property::Header as _;

use slog::Logger;

pub fn handle_input(
    info: &TokioServiceInfo,
    blockchain: &BlockchainR,
    _stats_counter: &StatsCounter,
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

    let logger = info.logger().clone();

    match bquery {
        BlockMsg::LeadershipExpectEndOfEpoch(epoch) => {
            let blockchain = blockchain.lock_read();
            chain::handle_end_of_epoch_event(&blockchain, epoch)
                .map_err(|e| crit!(logger, "end of epoch processing failed: {:?}", e))?;
        }
        BlockMsg::LeadershipBlock(block) => {
            let mut blockchain = blockchain.lock_write();
            match chain::handle_block(&mut blockchain, block, true)
                .map_err(|e| crit!(logger, "block processing failed: {:?}", e))?
            {
                HandledBlock::Rejected { reason } => {
                    warn!(logger,
                        "rejecting node's created block" ;
                        "reason" => reason.to_string(),
                    );
                }
                HandledBlock::MissingBranchToBlock { to } => {
                    // this is an error because we are in a situation
                    // where the leadership has created a block but we
                    // cannot add it in the blockchain because it is not
                    // connected
                    //
                    // We might want to stop the node at this point as this
                    // display corruption of the blockchain's state or of the
                    // storage
                    error!(
                        logger,
                        "the block cannot be added, missing intermediate blocks to {}", to
                    );
                }
                HandledBlock::Acquired { header } => {
                    info!(logger,
                        "block added successfully to Node's blockchain";
                        "id" => header.id().to_string(),
                        "date" => header.date().to_string()
                    );
                    debug!(logger, "Header: {:?}", header);
                    network_msg_box
                        .try_send(NetworkMsg::Propagate(PropagateMsg::Block(header)))
                        .unwrap_or_else(|err| {
                            error!(logger, "cannot propagate block to network: {}", err)
                        });
                }
            }
        }
        BlockMsg::NetworkBlock(block) => {
            let mut blockchain = blockchain.lock_write();
            match chain::handle_block(&mut blockchain, block, true).unwrap() {
                HandledBlock::Rejected { reason } => {
                    // TODO: drop the network peer that has sent
                    // an invalid block.
                    warn!(logger, "rejecting block from the network: {:?}", reason);
                }
                HandledBlock::MissingBranchToBlock { to } => {
                    // This is abnormal because we have received a block
                    // that is not connected to preceding blocks, which
                    // should not happen as we solicit blocks in descending
                    // order.
                    //
                    // TODO: drop the network peer that has sent
                    // the wrong block.
                    warn!(
                        logger,
                        "disconnected block received, missing intermediate blocks to {}", to
                    );
                }
                HandledBlock::Acquired { header } => {
                    info!(logger,
                        "block added successfully to Node's blockchain";
                        "id" => header.id().to_string(),
                        "date" => format!("{}.{}", header.date().epoch, header.date().slot_id)
                    );
                    debug!(logger, "Header: {:?}", header);
                    // Propagate the block to other nodes
                    network_msg_box
                        .try_send(NetworkMsg::Propagate(PropagateMsg::Block(header)))
                        .unwrap_or_else(|err| {
                            error!(logger, "cannot propagate block to network: {}", err)
                        });
                }
            }
        }
        BlockMsg::AnnouncedBlock(header, node_id) => {
            let blockchain = blockchain.lock_read();
            match chain::header_triage(&blockchain, &header, false).unwrap() {
                BlockHeaderTriage::NotOfInterest { reason } => {
                    info!(logger, "rejecting block announcement: {:?}", reason);
                }
                BlockHeaderTriage::MissingParentOrBranch { to } => {
                    // Blocks are missing between the received header and the
                    // common ancestor.
                    info!(
                        logger,
                        "received a loose block ({}), missing parent(s) block(s)", to
                    );
                    let from = blockchain.get_checkpoints().unwrap();
                    network_msg_box
                        .try_send(NetworkMsg::PullHeaders { node_id, from, to })
                        .unwrap_or_else(|err| {
                            error!(
                                logger,
                                "cannot send PullHeaders request to network: {}", err
                            )
                        });
                }
                BlockHeaderTriage::ProcessBlockToState => {
                    info!(logger, "Block announcement is interesting, fetch block");
                    network_msg_box
                        .try_send(NetworkMsg::GetNextBlock(node_id, header.id()))
                        .unwrap_or_else(|err| {
                            error!(
                                logger,
                                "cannot send GetNextBlock request to network: {}", err
                            )
                        });
                }
            }
        }
        BlockMsg::ChainHeaders(headers, reply) => {
            let res = process_chain_headers_into_block_request(blockchain, headers, &logger).map(
                |block_ids| {
                    network_msg_box
                        .try_send(NetworkMsg::GetBlocks(block_ids))
                        .unwrap_or_else(|err| {
                            error!(logger, "cannot send GetBlocks request to network: {}", err)
                        });
                },
            );
            reply.reply(res);
        }
    };

    Ok(())
}

fn process_chain_headers_into_block_request(
    blockchain: &BlockchainR,
    headers: Vec<Header>,
    logger: &Logger,
) -> Result<Vec<HeaderHash>, intercom::Error> {
    let blockchain = blockchain.lock_read();
    let mut block_ids = Vec::new();
    for header in headers {
        let triage = chain::header_triage(&blockchain, &header, false).map_err(|e| {
            info!(logger, "triage of pulled header failed: {:?}", e);
            intercom::Error::failed(e)
        })?;
        match triage {
            BlockHeaderTriage::ProcessBlockToState => {
                block_ids.push(header.hash());
            }
            BlockHeaderTriage::NotOfInterest { reason } => {
                // The block is already present, or is otherwise of no
                // interest. We cancel streaming of the entire branch.
                info!(
                    logger,
                    "pulled block header {} is not of interest: {:?}",
                    header.hash(),
                    reason,
                );
                return Err(intercom::Error::failed_precondition(format!(
                    "block {} is not accepted: {}",
                    header.hash(),
                    reason,
                )));
            }
            BlockHeaderTriage::MissingParentOrBranch { .. } => {
                info!(
                    logger,
                    "pulled block header {} is not connected to the blockchain",
                    header.hash(),
                );
                return Err(intercom::Error::failed_precondition(format!(
                    "block {} is not connected to the blockchain",
                    header.hash(),
                )));
            }
        }
    }
    Ok(block_ids)
}
