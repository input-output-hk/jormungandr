use crate::intercom::{self, ReplyFuture, ReplyHandle};
use crate::utils::async_msg::MessageBox;
use network_core::error as core_error;

use futures::prelude::*;
use futures::sink::Send;
use slog::Logger;

use std::mem;

pub struct InboundProcessing<Msg> {
    state: State<Msg>,
    reply_future: Option<ReplyFuture<(), core_error::Error>>,
}

enum State<Msg> {
    Sending(Send<MessageBox<Msg>>),
    WaitingForReply,
    Error(core_error::Error),
    Gone,
}

impl<Msg> InboundProcessing<Msg> {
    pub fn with_unary<F>(msg_box: MessageBox<Msg>, logger: Logger, f: F) -> Self
    where
        F: FnOnce(ReplyHandle<()>) -> Msg,
    {
        let (reply, reply_future) = intercom::unary_reply(logger);
        let msg = f(reply);
        let send = msg_box.send(msg);
        InboundProcessing {
            state: State::Sending(send),
            reply_future: Some(reply_future),
        }
    }

    pub fn error(err: core_error::Error) -> Self {
        InboundProcessing {
            state: State::Error(err),
            reply_future: None,
        }
    }
}

impl<Msg> Future for InboundProcessing<Msg> {
    type Item = ();
    type Error = core_error::Error;

    fn poll(&mut self) -> Poll<(), core_error::Error> {
        loop {
            match self.state {
                State::Sending(ref mut future) => {
                    try_ready!(future.poll().map_err(|_| core_error::Error::new(
                        core_error::Code::Aborted,
                        "the node stopped processing incoming items",
                    )));
                    self.state = State::WaitingForReply;
                }
                State::WaitingForReply => {
                    let future = self.reply_future.as_mut().unwrap();
                    try_ready!(future.poll());
                    self.state = State::Gone;
                    return Ok(().into());
                }
                State::Error(_) => {
                    if let State::Error(e) = mem::replace(&mut self.state, State::Gone) {
                        return Err(e);
                    } else {
                        unreachable!();
                    }
                }
                State::Gone => panic!("polled a finished future"),
            }
        }
    }
}
