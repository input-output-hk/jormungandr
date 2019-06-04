//! Blockchain content service abstraction.

use super::P2pService;
use crate::error::Error;

use chain_core::property::{Message, MessageId};

use futures::prelude::*;

/// Interface for the blockchain node service implementation responsible for
/// validating and accepting transactions and other block contents, known
/// together as messages.
pub trait ContentService: P2pService {
    /// The data type to represent messages constituting a block.
    type Message: Message;

    /// The message identifier type for the blockchain.
    type MessageId: MessageId;

    /// The type of an asynchronous stream that provides message contents in
    /// response to `get_messages`.
    type GetMessagesStream: Stream<Item = Self::Message, Error = Error> + Send + 'static;

    /// The type of asynchronous futures returned by `get_messages`.
    ///
    /// The future resolves to a stream that will be used by the protocol
    /// implementation to produce a server-streamed response.
    type GetMessagesFuture: Future<Item = Self::GetMessagesStream, Error = Error>;

    /// The type of an asynchronous stream that provides transactions announced
    /// by the peer via the bidirectional subscription.
    type MessageSubscription: Stream<Item = Self::Message, Error = Error> + Send + 'static;

    /// The type of asynchronous futures returned by method `message_subscription`.
    ///
    /// The future resolves to a stream that will be used by the protocol
    /// implementation to produce a server-streamed response.
    type MessageSubscriptionFuture: Future<Item = Self::MessageSubscription, Error = Error>;

    /// Get all transactions by their id.
    fn get_messages(&mut self, ids: &[Self::MessageId]) -> Self::GetMessagesFuture;

    /// Establishes a bidirectional subscription for announcing new messages.
    ///
    /// The network protocol implementation passes the node identifier of
    /// the sender and an asynchronous stream that will provide the inbound
    /// announcements.
    ///
    /// Returns a future resolving to an asynchronous stream
    /// that will be used by this node to send message announcements.
    fn message_subscription<In>(
        &mut self,
        subscriber: Self::NodeId,
        inbound: In,
    ) -> Self::MessageSubscriptionFuture
    where
        In: Stream<Item = Self::Message, Error = Error> + Send + 'static;
}
