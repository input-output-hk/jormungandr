use super::{p2p_topology::Node, GlobalStateR};
use crate::{blockcfg::Header, intercom::BlockMsg, utils::async_msg::MessageBox};

use network_core::{error as core_error, gossip::Gossip};

use futures::prelude::*;

pub fn process_blocks<S>(inbound: S, mut block_box: MessageBox<BlockMsg>) -> tokio::executor::Spawn
where
    S: Stream<Item = Header, Error = core_error::Error> + Send + 'static,
{
    tokio::spawn(
        inbound
            .for_each(move |header| {
                block_box.send(BlockMsg::AnnouncedBlock(header));
                Ok(())
            })
            .map_err(|err| {
                error!("block subscription stream failure: {:?}", err);
            }),
    )
}

pub fn process_gossip<S>(inbound: S, state: GlobalStateR) -> tokio::executor::Spawn
where
    S: Stream<Item = Gossip<Node>, Error = core_error::Error> + Send + 'static,
{
    tokio::spawn(
        inbound
            .for_each(move |gossip| {
                state.topology.update(gossip.into_nodes());
                Ok(())
            })
            .map_err(|err| {
                info!("gossip subscription stream failure: {:?}", err);
            }),
    )
}
