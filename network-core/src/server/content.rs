//! Blockchain content service abstraction.

use crate::error::Error;

use chain_core::property::{Deserialize, Message, MessageId, Serialize};

use futures::prelude::*;

/// Interface for the blockchain node service implementation responsible for
/// validating and accepting transactions and other block contents, known
/// together as messages.
pub trait ContentService {
    /// The data type to represent messages constituting a block.
    type Message: Message + Serialize;

    /// The message identifier type for the blockchain.
    type MessageId: MessageId + Serialize + Deserialize;

    /// The type of asynchronous futures returned by method `propose_transactions`.
    type ProposeTransactionsFuture: Future<
        Item = ProposeTransactionsResponse<Self::MessageId>,
        Error = Error,
    >;

    /// The type of an asynchronous stream that provides message contents in
    /// response to `get_messages`.
    type GetMessagesStream: Stream<Item = Self::Message, Error = Error>;

    /// The type of asynchronous futures returned by `get_messages`.
    ///
    /// The future resolves to a stream that will be used by the protocol
    /// implementation to produce a server-streamed response.
    type GetMessagesFuture: Future<Item = Self::GetMessagesStream, Error = Error>;

    /// The type of an asynchronous stream that provides transactions announced
    /// by the peer via the bidirectional subscription.
    type MessageSubscription: Stream<Item = Self::Message, Error = Error>;

    /// The type of asynchronous futures returned by method `message_subscription`.
    ///
    /// The future resolves to a stream that will be used by the protocol
    /// implementation to produce a server-streamed response.
    type MessageSubscriptionFuture: Future<Item = Self::MessageSubscription, Error = Error>;

    /// Get all transactions by their id.
    fn get_messages(&mut self, ids: &[Self::MessageId]) -> Self::GetMessagesFuture;

    /// Given a list of transaction IDs, return status of the transactions
    /// as known by this node.
    ///
    /// This method is only used by the NTT implementation.
    fn propose_transactions(&mut self, ids: &[Self::MessageId]) -> Self::ProposeTransactionsFuture;

    // Establishes a bidirectional subscription for announcing new messages,
    // taking an asynchronous stream that provides the inbound announcements.
    //
    // Returns a future that resolves to an asynchronous subscription stream
    // that receives messages announced by the peer.
    fn message_subscription<In>(&mut self, inbound: In) -> Self::MessageSubscriptionFuture
    where
        In: Stream<Item = Self::Message, Error = Error>;
}

/// Response from the `propose_transactions` method of a `TransactionService`.
pub struct ProposeTransactionsResponse<Id> {
    // TODO: define fully
    _ids: Vec<Id>,
}
