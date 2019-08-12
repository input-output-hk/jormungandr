use crate::{
    blockchain::{Blockchain, Branch, PreCheckedHeader},
    intercom::{BlockMsg, NetworkMsg},
    leadership::NewEpochToSchedule,
    stats_counter::StatsCounter,
    utils::{
        async_msg::MessageBox,
        task::{Input, TokioServiceInfo},
    },
};
use chain_core::property::HasHeader as _;
use tokio::{prelude::*, sync::mpsc::Sender};

pub fn handle_input(
    info: &TokioServiceInfo,
    blockchain: &mut Blockchain,
    blockchain_tip: &mut Branch,
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
        BlockMsg::LeadershipExpectEndOfEpoch(epoch) => unimplemented!(),
        BlockMsg::LeadershipBlock(block) => {
            let header = block.header();

            match blockchain.pre_check_header(header).wait().unwrap() {
                PreCheckedHeader::HeaderWithCache { header, parent_ref } => {
                    let pch = blockchain
                        .post_check_header(header, parent_ref)
                        .wait()
                        .unwrap();
                    let new_block_ref = blockchain.apply_block(pch, block).wait().unwrap();

                    blockchain_tip.update_ref(new_block_ref).wait().unwrap();
                }
                _ => unimplemented!(),
            }
        }
        BlockMsg::AnnouncedBlock(header, node_id) => unimplemented!(),
        BlockMsg::NetworkBlock(block, reply) => unimplemented!(),
        BlockMsg::ChainHeaders(headers, reply) => unimplemented!(),
    };

    Ok(())
}
