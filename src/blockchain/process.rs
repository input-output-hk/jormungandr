use crate::blockchain::chain::{self, BlockHeaderTriage, BlockchainR, HandledBlock};
use crate::intercom::{BlockMsg, NetworkPropagateMsg};
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
    network_propagate: &MessageBox<NetworkPropagateMsg>,
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
                    network_propagate
                        .clone()
                        .send(NetworkPropagateMsg::Block(header));
                }
            }
        }
        BlockMsg::AnnouncedBlock(header) => {
            let blockchain = blockchain.lock_read();
            match chain::header_triage(&blockchain, header, false).unwrap() {
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
                    // TODO: signal back to the network that the block is interesting
                    // (get block/request block)
                    unimplemented!()
                }
            }
        }
    }
}
