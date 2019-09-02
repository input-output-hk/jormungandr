use crate::intercom::{self, ReplyFuture, ReplyHandle};
use crate::utils::async_msg::MessageBox;
use network_core::error as core_error;

use futures::prelude::*;
use futures::sink;
use slog::Logger;

use std::mem;

pub type MsgFunc<T, Msg> = fn(T, ReplyHandle<()>) -> Msg;

pub struct InboundProcessing<T, Msg> {
    state: State<Msg>,
    conv: MsgFunc<T, Msg>,
    logger: Logger,
}

enum State<Msg> {
    Ready(MessageBox<Msg>),
    Sending {
        future: sink::Send<MessageBox<Msg>>,
        reply: ReplyFuture<(), core_error::Error>,
    },
    WaitingReply {
        reply: ReplyFuture<(), core_error::Error>,
        mbox: MessageBox<Msg>,
    },
    Transitional,
}

impl<Msg> State<Msg> {
    fn poll_ready(&mut self) -> Poll<(), core_error::Error> {
        loop {
            let mbox_from_send = match self {
                State::Ready(_) => return Ok(().into()),
                State::Sending { future, .. } => {
                    let mbox = try_ready!(future.poll().map_err(|_| core_error::Error::new(
                        core_error::Code::Aborted,
                        "the node stopped processing incoming items",
                    )));
                    Some(mbox)
                }
                State::WaitingReply { reply, .. } => {
                    try_ready!(reply.poll());
                    None
                }
                State::Transitional => unreachable!(),
            };
            *self = match mem::replace(self, State::Transitional) {
                State::Ready(_) => unreachable!(),
                State::Sending { reply, .. } => State::WaitingReply {
                    reply,
                    mbox: mbox_from_send.unwrap(),
                },
                State::WaitingReply { mbox, .. } => State::Ready(mbox),
                State::Transitional => unreachable!(),
            }
        }
    }
}

impl<T, Msg> InboundProcessing<T, Msg> {
    pub fn with_unary(mbox: MessageBox<Msg>, logger: Logger, f: MsgFunc<T, Msg>) -> Self {
        InboundProcessing {
            state: State::Ready(mbox),
            conv: f,
            logger,
        }
    }
}

impl<T, Msg> Sink for InboundProcessing<T, Msg> {
    type SinkItem = T;
    type SinkError = core_error::Error;

    fn start_send(&mut self, item: T) -> StartSend<T, Self::SinkError> {
        match self.state.poll_ready()? {
            Async::NotReady => return Ok(AsyncSink::NotReady(item)),
            Async::Ready(()) => {}
        };
        let mbox = match mem::replace(&mut self.state, State::Transitional) {
            State::Ready(mbox) => mbox,
            _ => unreachable!(),
        };
        let (reply_handle, reply_future) = intercom::unary_reply(self.logger.clone());
        let msg = (self.conv)(item, reply_handle);
        let future = mbox.send(msg);
        self.state = State::Sending {
            future,
            reply: reply_future,
        };
        Ok(AsyncSink::Ready)
    }

    fn poll_complete(&mut self) -> Poll<(), Self::SinkError> {
        self.state.poll_ready()
    }
}
