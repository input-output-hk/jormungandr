use crate::{
    blockchain::Blockchain,
    intercom::{BlockMsg, NetworkMsg},
    stats_counter::StatsCounter,
    utils::{
        async_msg::MessageBox,
        task::{Input, TokioServiceInfo},
    }
};

pub fn handle_input(
    info: &TokioServiceInfo,
    blockchain: &Blockchain,
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

    match bquery {
        BlockMsg::LeadershipExpectEndOfEpoch(epoch) => {
            unimplemented!()
        }
        BlockMsg::LeadershipBlock(block) => {
            unimplemented!()
        }
        BlockMsg::AnnouncedBlock(header, node_id) => {
            unimplemented!()
        }
        BlockMsg::NetworkBlock(block, reply) => {
            unimplemented!()
        }
        BlockMsg::ChainHeaders(headers, reply) => {
            unimplemented!()
        }
    };

    Ok(())
}