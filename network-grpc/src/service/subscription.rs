use super::{request_stream::Forward, response_stream};
use crate::convert::{encode_node_id, error_into_grpc, FromProtobuf, IntoProtobuf};
use chain_core::property;
use network_core::error as core_error;
use network_core::gossip::NodeId;
use network_core::server::request_stream::MapResponse;

use futures::prelude::*;
use tower_grpc::{self, Code, Status};

use std::marker::PhantomData;
use std::mem;

pub struct Subscription<T, In, S>
where
    S: Sink + MapResponse,
{
    state: State<In, S>,
    _phantom: PhantomData<T>,
}

enum State<In, S>
where
    S: Sink + MapResponse,
{
    Full(Forward<In, S>),
    InboundClosed {
        outbound: Option<S>,
        shutdown: S::ResponseFuture,
    },
    OutboundGone {
        sink: S,
    },
}

impl<In, S> State<In, S>
where
    S: Stream + Sink + MapResponse,
{
    fn outbound_stream(&mut self) -> Option<&mut S> {
        match self {
            State::Full(forward) => Some(forward.sink_mut()),
            State::InboundClosed { outbound, .. } => outbound.as_mut(),
            State::OutboundGone { .. } => None,
        }
    }
}

impl<T, In, S> Subscription<T, In, S>
where
    In: Stream,
    S: Sink + MapResponse,
{
    fn new(inbound: In, core_subscription: S) -> Self {
        let forward = Forward::new(inbound, core_subscription);
        Subscription {
            state: State::Full(forward),
            _phantom: PhantomData,
        }
    }
}

impl<T, In, S> Subscription<T, In, S>
where
    In: Stream<Error = Status>,
    S: Sink<SinkError = core_error::Error>,
    S: MapResponse,
    S::SinkItem: FromProtobuf<In::Item>,
{
    fn process_inbound(&mut self) -> Poll<Option<()>, Status> {
        match &mut self.state {
            State::Full(forward) => match forward.poll_step().unwrap() {
                Async::NotReady => Ok(Async::NotReady),
                Async::Ready(None) => Ok(None.into()),
                Async::Ready(Some((outbound, shutdown))) => {
                    self.state = State::InboundClosed {
                        outbound: Some(outbound),
                        shutdown,
                    };
                    Ok(None.into())
                }
            },
            State::InboundClosed { shutdown, .. } => {
                try_ready!(shutdown.poll().map_err(error_into_grpc));
                Ok(Some(()).into())
            }
            State::OutboundGone { sink } => {
                try_ready!(sink.close().map_err(error_into_grpc));
                Ok(Some(()).into())
            }
        }
    }

    fn drop_outbound(&mut self) {
        match &mut self.state {
            State::Full(forward) => {
                let sink = forward.break_up();
                self.state = State::OutboundGone { sink };
            }
            State::InboundClosed {
                ref mut outbound, ..
            } => {
                *outbound = None;
            }
            State::OutboundGone { .. } => {
                unreachable!("should not poll None more than once from the outbound stream",)
            }
        }
    }
}

impl<T, In, S> Stream for Subscription<T, In, S>
where
    In: Stream<Error = Status>,
    S: Stream<Error = core_error::Error>,
    S: Sink<SinkError = core_error::Error>,
    S: MapResponse,
    S::Item: IntoProtobuf<T>,
    S::SinkItem: FromProtobuf<In::Item>,
{
    type Item = T;
    type Error = Status;

    fn poll(&mut self) -> Poll<Option<T>, Status> {
        loop {
            if let Some(stream) = self.state.outbound_stream() {
                match response_stream::poll_and_convert(stream)? {
                    Async::NotReady => {
                        // Let inbound processing decide
                        // if the whole thing is ready.
                    }
                    Async::Ready(Some(item)) => {
                        // Make sure inbound is processed in turn and
                        // handle termination, but otherwise
                        // don't worry if it's not ready as we have an
                        // item to return.
                        match self.process_inbound()? {
                            Async::Ready(Some(())) => return Ok(None.into()),
                            Async::NotReady | Async::Ready(None) => {}
                        }
                        return Ok(Some(item).into());
                    }
                    Async::Ready(None) => {
                        // As per RFC 7540 section 8.1, the stream is
                        // closed after the server ends the response.
                        // Stop the inbound forwarding and begin closing
                        // its sink.
                        self.drop_outbound();
                    }
                }
            }
            match try_ready!(self.process_inbound()) {
                None => continue,
                Some(()) => return Ok(None.into()),
            }
        }
    }
}

#[must_use = "futures do nothing unless polled"]
pub enum SubscriptionFuture<T, In, Id, F> {
    Normal { inner: F, inbound: In, node_id: Id },
    Failed(Status),
    Finished(PhantomData<T>),
}

impl<T, In, Id, F> SubscriptionFuture<T, In, Id, F> {
    pub fn new(node_id: Id, inbound: In, core_subscription: F) -> Self {
        SubscriptionFuture::Normal {
            inner: core_subscription,
            inbound,
            node_id,
        }
    }

    pub fn error(status: Status) -> Self {
        SubscriptionFuture::Failed(status)
    }

    pub fn unimplemented() -> Self {
        SubscriptionFuture::Failed(Status::new(Code::Unimplemented, "not implemented"))
    }
}

impl<T, In, Id, F> Future for SubscriptionFuture<T, In, Id, F>
where
    Id: NodeId + property::Serialize,
    In: Stream<Error = Status>,
    F: Future<Error = core_error::Error>,
    F::Item: Sink<SinkError = core_error::Error>,
    F::Item: MapResponse,
{
    type Item = tower_grpc::Response<Subscription<T, In, F::Item>>;
    type Error = Status;

    fn poll(&mut self) -> Poll<Self::Item, Status> {
        use SubscriptionFuture::*;

        let core_subscription = match self {
            Normal { inner, .. } => {
                let sub = try_ready!(inner.poll().map_err(error_into_grpc));
                Some(sub)
            }
            _ => None,
        };
        match mem::replace(self, Finished(PhantomData)) {
            Normal {
                inbound, node_id, ..
            } => {
                let subscription = Subscription::new(inbound, core_subscription.unwrap());
                let mut res = tower_grpc::Response::new(subscription);
                encode_node_id(&node_id, res.metadata_mut())?;
                Ok(Async::Ready(res))
            }
            Failed(status) => Err(status),
            Finished(_) => panic!("polled a finished subscription future"),
        }
    }
}
