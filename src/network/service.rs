use super::ConnectionState;
use crate::blockcfg::BlockConfig;
use crate::intercom::{stream_reply, unary_reply, ClientMsg, ReplyFuture, ReplyStream};
use crate::utils::task::TaskMessageBox;

use network_core::server::block::{BlockService, BlockError};

use futures::future::{self, FutureResult};

pub struct ConnectionBlockService<B: BlockConfig> {
    pub client_box: TaskMessageBox<ClientMsg<B>>,
}

impl<B: BlockConfig> ConnectionBlockService<B> {
    pub fn new(conn: &ConnectionState) -> Self {
        ConnectionBlockService {
            client_box: conn.channels.client_box.clone(),
        }
    }
}

impl<B: BlockConfig> BlockService for ConnectionBlockService<B> {
    type BlockId = B::BlockHash;
    type BlockDate = B::BlockDate;
    type Block = B::Block;
    type TipFuture = ReplyFuture<(Self::BlockId, Self::BlockDate), BlockError>;
    type GetBlocksStream = ReplyStream<B::Block, BlockError>;
    type GetBlocksFuture = FutureResult<Self::GetBlocksStream, BlockError>;
    type PullBlocksToTipStream = ReplyStream<B::Block, BlockError>;
    type PullBlocksFuture = FutureResult<Self::PullBlocksToTipStream, BlockError>;

    fn tip(&mut self) -> Self::TipFuture {
        let (handle, future) = unary_reply();
        self.client_box.send_to(ClientMsg::GetBlockTip(handle));
        future
    }

    fn pull_blocks_to_tip(&mut self, from: &[Self::BlockId]) -> Self::PullBlocksFuture {
        let (handle, stream) = stream_reply();
        self.client_box
            .send_to(ClientMsg::PullBlocksToTip(from.into(), handle));
        future::ok(stream)
    }

    fn pull_blocks_to(
        &mut self,
        from: &[Self::BlockId],
        to: &Self::BlockId,
    ) -> Self::PullBlocksFuture {
        unimplemented!()
    }
}
