use intercom::{Error, Reply, StreamReply};

use blockcfg::{Block, Header};
use protocol::{protocol, network_transport::LightWeightConnectionId};
use futures::sync::mpsc::UnboundedSender;

/// Simple RAII for the reply information to NTT protocol commands
#[derive(Clone, Debug)]
pub struct ReplyHandle {
    // the identifier of the connection we are replying to
    identifier: LightWeightConnectionId,
    // the appropriate sink to send the messages to
    sink: UnboundedSender<protocol::Message>,
    closed: bool,
}

impl ReplyHandle {
    pub fn new(
        identifier: LightWeightConnectionId,
        sink: UnboundedSender<protocol::Message>,
    ) -> Self {
        ReplyHandle { identifier, sink, closed: false }
    }

    fn send_message(&self, message: protocol::Message) {
        debug_assert!(!self.closed);
        self.sink.unbounded_send(message).unwrap();
    }

    fn send_close(&mut self) {
        debug_assert!(!self.closed);
        self.sink.unbounded_send(
            protocol::Message::CloseConnection(self.identifier)
        ).unwrap();
        self.closed = true;
    }
}

impl Drop for ReplyHandle {
    fn drop(&mut self) {
        if !self.closed {
            warn!("protocol reply was not properly finalized");
            self.sink.unbounded_send(
                protocol::Message::CloseConnection(self.identifier)
            ).unwrap_or_default();
        }
    }
}

impl Reply<Vec<Header>> for ReplyHandle {
    fn reply_ok(&mut self, item: Vec<Header>) {
        self.send_message(
            protocol::Message::BlockHeaders(
                self.identifier,
                protocol::Response::Ok(item.into()),
            )
        );
        self.send_close();
    }

    fn reply_error(&mut self, error: Error) {
        self.send_message(
            protocol::Message::BlockHeaders(
                self.identifier,
                protocol::Response::Err(error.to_string()),
            )
        );
        self.send_close();
    }
}

impl Reply<Header> for ReplyHandle {
    fn reply_ok(&mut self, item: Header) {
        self.send_message(
            protocol::Message::BlockHeaders(
                self.identifier,
                protocol::Response::Ok(protocol::BlockHeaders(vec![item])),
            )
        );
        self.send_close();
    }

    fn reply_error(&mut self, error: Error) {
        self.send_message(
            protocol::Message::BlockHeaders(
                self.identifier,
                protocol::Response::Err(error.to_string()),
            )
        );
        self.send_close();
    }
}

impl StreamReply<Block> for ReplyHandle {
    fn send(&mut self, blk: Block) {
        self.send_message(
            protocol::Message::Block(
                self.identifier,
                protocol::Response::Ok(blk),
            )
        );
    }

    fn send_error(&mut self, error: Error) {
        self.send_message(
            protocol::Message::Block(
                self.identifier,
                protocol::Response::Err(error.to_string()),
            )
        );
    }

    fn close(&mut self) {
        self.send_close()
    }
}
