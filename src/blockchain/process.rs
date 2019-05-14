use crate::blockchain::chain::{self, BlockHeaderTriage, BlockchainR, HandledBlock};
use crate::intercom::{BlockMsg, NetworkMsg, PropagateMsg};
use crate::rest::v0::node::stats::StatsCounter;
use crate::utils::{
    async_msg::MessageBox,
    task::{Input, TokioServiceInfo},
};

use chain_core::property::Header as _;

pub fn handle_input(
    info: &TokioServiceInfo,
    blockchain: &BlockchainR,
    _stats_counter: &StatsCounter,
    network_msg_box: &mut MessageBox<NetworkMsg>,
    input: Input<BlockMsg>,
) {
    let bquery = match input {
        Input::Shutdown => {
            // TODO: is there some work to do here to clean up the
            //       the state and make sure all state is saved properly
            return;
        }
        Input::Input(msg) => msg,
    };

    let logger = info.logger().clone();

    match bquery {
        BlockMsg::LeadershipExpectEndOfEpoch => {
            let blockchain = blockchain.lock_read();
            chain::handle_end_of_epoch_event(&blockchain).unwrap()
        }
        BlockMsg::LeadershipBlock(block) => {
            let mut blockchain = blockchain.lock_write();
            match chain::handle_block(&mut blockchain, block, true).unwrap() {
                HandledBlock::Rejected { reason } => {
                    slog_warn!(logger, "rejecting node's created block: {:?}", reason);
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
                    slog_error!(
                        logger,
                        "the block cannot be added, missing intermediate blocks to {}",
                        to
                    );
                }
                HandledBlock::Acquired { header } => {
                    slog_info!(logger,
                        "block added successfully to Node's blockchain";
                        "id" => header.id().to_string(),
                        "date" => header.date().to_string()
                    );
                    slog_debug!(logger, "Header: {:?}", header);
                    network_msg_box.send(NetworkMsg::Propagate(PropagateMsg::Block(header)));
                }
            }
        }
        BlockMsg::NetworkBlock(block) => {
            let mut blockchain = blockchain.lock_write();
            match chain::handle_block(&mut blockchain, block, true).unwrap() {
                HandledBlock::Rejected { reason } => {
                    // TODO: drop the network peer that has sent
                    // an invalid block.
                    slog_warn!(logger, "rejecting block from the network: {:?}", reason);
                }
                HandledBlock::MissingBranchToBlock { to } => {
                    // This is abnormal because we have received a block
                    // that is not connected to preceding blocks, which
                    // should not happen as we solicit blocks in descending
                    // order.
                    //
                    // TODO: drop the network peer that has sent
                    // the wrong block.
                    slog_warn!(
                        logger,
                        "disconnected block received, missing intermediate blocks to {}",
                        to
                    );
                }
                HandledBlock::Acquired { header } => {
                    slog_info!(logger,
                        "block added successfully to Node's blockchain";
                        "id" => header.id().to_string(),
                        "date" => format!("{}.{}", header.date().epoch, header.date().slot_id)
                    );
                    slog_debug!(logger, "Header: {:?}", header);
                    // Propagate the block to other nodes
                    network_msg_box.send(NetworkMsg::Propagate(PropagateMsg::Block(header)));
                }
            }
        }
        BlockMsg::AnnouncedBlock(header, node_id) => {
            let blockchain = blockchain.lock_read();
            match chain::header_triage(&blockchain, &header, false).unwrap() {
                BlockHeaderTriage::NotOfInterest { reason } => {
                    slog_info!(logger, "rejecting block announcement: {:?}", reason);
                }
                BlockHeaderTriage::MissingParentOrBranch { to } => {
                    // blocks are missing between the received header and the
                    // common ancestor.
                    //
                    // TODO reply to the network to ask for more blocks
                    slog_info!(
                        logger,
                        "received a loose block ({}), missing parent(s) block(s)",
                        to
                    );
                }
                BlockHeaderTriage::ProcessBlockToState => {
                    slog_info!(logger, "Block announcement is interesting, fetch block");
                    network_msg_box.send(NetworkMsg::GetBlocks(node_id, vec![header.id()]));
                }
            }
        }
    }
}
