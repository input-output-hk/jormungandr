use crate::convert::{error_from_grpc, error_into_grpc, FromProtobuf, IntoProtobuf};
use network_core::error as core_error;
use network_core::server::request_stream::{MapResponse, ProcessingError};

use futures::prelude::*;
use futures::stream::Fuse;
use tower_grpc::{Code, Status};

use std::convert::Infallible;
use std::hint::unreachable_unchecked;
use std::marker::PhantomData;
use std::mem;

/// Low level implementation of request stream forwarding to a sink.
#[must_use = "futures do nothing unless polled"]
pub struct Forward<In, S>
where
    S: Sink,
{
    inbound: Fuse<In>,
    sink: Option<S>,
    buffered: Option<S::SinkItem>,
}

#[must_use = "futures do nothing unless polled"]
pub enum Processing<In, S, R>
where
    S: Sink + MapResponse,
{
    Forwarding(Forward<In, S>),
    PendingResponse(S::ResponseFuture),
    Failed(Status),
    Finished(PhantomData<R>),
}

impl<In: Stream, S: Sink> Forward<In, S> {
    pub fn new(inbound: In, sink: S) -> Self {
        Forward {
            inbound: inbound.fuse(),
            sink: Some(sink),
            buffered: None,
        }
    }
}

impl<In, S: Sink> Forward<In, S> {
    pub fn sink_mut(&mut self) -> &mut S {
        self.sink
            .as_mut()
            .expect("attempted to poll request stream processing after completion")
    }

    pub fn break_up(&mut self) -> S {
        self.sink
            .take()
            .expect("can't break down stream forwarding twice")
    }
}

impl<In, S> Forward<In, S>
where
    S: Sink<SinkError = core_error::Error>,
{
    fn try_send_item(&mut self, item: S::SinkItem) -> Poll<(), ProcessingError> {
        match self
            .sink_mut()
            .start_send(item)
            .map_err(ProcessingError::Sink)?
        {
            AsyncSink::Ready => Ok(Async::Ready(())),
            AsyncSink::NotReady(item) => {
                debug_assert!(self.buffered.is_none());
                self.buffered = Some(item);
                Ok(Async::NotReady)
            }
        }
    }
}

impl<In, S> Forward<In, S>
where
    In: Stream<Error = Status>,
    S: Sink<SinkError = core_error::Error>,
    S::SinkItem: FromProtobuf<In::Item>,
{
    fn poll_step_internal(&mut self) -> Poll<Option<()>, ProcessingError> {
        if let Some(item) = self.buffered.take() {
            try_ready!(self.try_send_item(item));
            Ok(None.into())
        } else {
            match self
                .inbound
                .poll()
                .map_err(|e| ProcessingError::Inbound(error_from_grpc(e)))?
            {
                Async::NotReady => {
                    try_ready!(self
                        .sink_mut()
                        .poll_complete()
                        .map_err(ProcessingError::Sink));
                    Ok(Async::NotReady)
                }
                Async::Ready(Some(msg)) => {
                    let item =
                        FromProtobuf::from_message(msg).map_err(ProcessingError::Decoding)?;
                    try_ready!(self.try_send_item(item));
                    Ok(None.into())
                }
                Async::Ready(None) => {
                    try_ready!(self.sink_mut().close().map_err(ProcessingError::Sink));
                    Ok(Some(()).into())
                }
            }
        }
    }
}

impl<In, S> Forward<In, S>
where
    In: Stream<Error = Status>,
    S: Sink<SinkError = core_error::Error>,
    S: MapResponse,
    S::SinkItem: FromProtobuf<In::Item>,
{
    pub fn poll_step(&mut self) -> Poll<Option<(S, S::ResponseFuture)>, Infallible> {
        let terminated = match self.poll_step_internal() {
            Ok(Async::NotReady) => return Ok(Async::NotReady),
            Ok(Async::Ready(None)) => return Ok(None.into()),
            Ok(Async::Ready(Some(()))) => Ok(()),
            Err(e) => Err(e),
        };
        let mut sink = self.sink.take().unwrap();
        let shutdown = sink.on_stream_termination(terminated);
        Ok(Some((sink, shutdown)).into())
    }
}

impl<In, S, R> Processing<In, S, R>
where
    In: Stream,
    S: Sink + MapResponse,
{
    pub fn new(inbound: In, sink: S) -> Self {
        let forward = Forward::new(inbound, sink);
        Processing::Forwarding(forward)
    }

    pub fn unimplemented() -> Self {
        Processing::Failed(Status::new(Code::Unimplemented, "not implemented"))
    }
}

impl<In, S, R> Future for Processing<In, S, R>
where
    In: Stream<Error = Status>,
    S: Sink<SinkError = core_error::Error>,
    S: MapResponse,
    S::SinkItem: FromProtobuf<In::Item>,
    S::Response: IntoProtobuf<R>,
{
    type Item = tower_grpc::Response<R>;
    type Error = Status;

    fn poll(&mut self) -> Poll<Self::Item, Status> {
        use Processing::*;
        loop {
            match self {
                Forwarding(forward) => match forward.poll_step().unwrap() {
                    Async::NotReady => return Ok(Async::NotReady),
                    Async::Ready(None) => {}
                    Async::Ready(Some((_sink, shutdown))) => {
                        *self = PendingResponse(shutdown);
                    }
                },
                PendingResponse(future) => {
                    let res = try_ready!(future.poll().map_err(error_into_grpc));
                    *self = Finished(PhantomData);
                    let res = res.into_message()?;
                    return Ok(tower_grpc::Response::new(res).into());
                }
                Failed(_) => {
                    if let Failed(status) = mem::replace(self, Finished(PhantomData)) {
                        return Err(status);
                    } else {
                        unsafe { unreachable_unchecked() }
                    }
                }
                Finished(_) => panic!("polled a finished request processing future"),
            }
        }
    }
}
