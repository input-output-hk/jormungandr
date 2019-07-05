use crate::blockcfg::Header;
use crate::intercom::{self, BlockMsg, ReplyFuture};
use crate::utils::async_msg::MessageBox;
use network_core::error as core_error;

use futures::prelude::*;
use futures::stream::Chunks;
use futures::sync::mpsc;
use slog::Logger;

// Size of chunks to split processing of chain pull streams.
// Apart from sizing data chunks for intercom messages, it also
// determines how many blocks will be requested per each GetBlocks request
// distributed between different peers.
//
// This may need to be made into a configuration parameter.
const CHUNK_SIZE: usize = 32;

/// State machine for pulling blocks from the network after processing
/// the stream of block headers composing the chain.
pub struct BlockPull<In>
where
    In: Stream<Item = Header, Error = core_error::Error>,
{
    in_chunks: Chunks<In>,
    block_box: MessageBox<BlockMsg>,
    chain_reply: Option<ReplyFuture<(), core_error::Error>>,
    state: State,
    logger: Logger,
}

enum State {
    ReadNext,
    SendingChunk(Option<BlockMsg>),
    WaitReply,
}

impl<In> BlockPull<In>
where
    In: Stream<Item = Header, Error = core_error::Error>,
{
    pub fn new(stream: In, block_box: MessageBox<BlockMsg>, logger: Logger) -> Self {
        BlockPull {
            in_chunks: stream.chunks(CHUNK_SIZE),
            block_box,
            chain_reply: None,
            state: State::ReadNext,
            logger,
        }
    }
}

impl<In> Future for BlockPull<In>
where
    In: Stream<Item = Header, Error = core_error::Error>,
{
    type Item = ();
    type Error = core_error::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        use self::State::*;

        loop {
            let new_state = match self.state {
                ReadNext => {
                    let chunk = match try_ready!(self.in_chunks.poll()) {
                        None => return Ok(Async::Ready(())),
                        Some(chunk) => chunk,
                    };
                    let (reply, reply_future) = intercom::unary_reply(self.logger.clone());
                    debug_assert!(self.chain_reply.is_none());
                    self.chain_reply = Some(reply_future);
                    SendingChunk(Some(BlockMsg::ChainHeaders(chunk, reply)))
                }
                SendingChunk(ref mut msg) => match self.block_box.start_send(msg.take().unwrap()) {
                    Ok(AsyncSink::NotReady(rejected_msg)) => {
                        *msg = Some(rejected_msg);
                        try_ready!(self
                            .block_box
                            .poll_complete()
                            .map_err(convert_intercom_send_error));
                        SendingChunk(msg.take())
                    }
                    Ok(AsyncSink::Ready) => WaitReply,
                    Err(e) => return Err(convert_intercom_send_error(e)),
                },
                WaitReply => {
                    self.block_box
                        .poll_complete()
                        .map_err(convert_intercom_send_error)?;
                    let future = self
                        .chain_reply
                        .as_mut()
                        .expect("the reply future should be initialized");
                    try_ready!(future.poll());
                    ReadNext
                }
            };
            self.state = new_state;
        }
    }
}

fn convert_intercom_send_error<T>(_: mpsc::SendError<T>) -> core_error::Error {
    core_error::Error::new(
        core_error::Code::Canceled,
        "the node stopped processing incoming headers",
    )
}
