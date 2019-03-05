///! Gossip service abstraction.
use crate::error::Code as ErrorCode;

use super::super::gossip::{Gossip, NodeId};

use futures::prelude::*;

use std::{error, fmt};

/// Intreface for the node discovery service implementation
/// in the p2p network.
pub trait GossipService {
    /// Gossip message content.
    type Message: Gossip;

    type MessageFuture: Future<Item = (NodeId, Self::Message), Error = GossipError>;

    /// Record and process gossip event.
    fn record_gossip(&mut self, node_id: NodeId, gossip: &Self::Message) -> Self::MessageFuture;
}

#[derive(Debug)]
pub struct GossipError {
    code: ErrorCode,
    cause: Option<Box<dyn error::Error + Send + Sync>>,
}

impl GossipError {
    pub fn failed<E>(cause: E) -> Self
    where
        E: Into<Box<dyn error::Error + Send + Sync>>,
    {
        GossipError {
            code: ErrorCode::Failed,
            cause: Some(cause.into()),
        }
    }

    pub fn with_code_and_cause<E>(code: ErrorCode, cause: E) -> Self
    where
        E: Into<Box<dyn error::Error + Send + Sync>>,
    {
        GossipError {
            code,
            cause: Some(cause.into()),
        }
    }

    pub fn code(&self) -> ErrorCode {
        self.code
    }
}

impl error::Error for GossipError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        if let Some(err) = &self.cause {
            Some(&**err)
        } else {
            None
        }
    }
}

impl fmt::Display for GossipError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "gossip service error: {}", self.code)
    }
}
