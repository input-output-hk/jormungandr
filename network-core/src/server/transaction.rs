//! Transaction service abstraction.

use crate::error::Error;

use chain_core::property::{Deserialize, Message, MessageId, Serialize};

use futures::prelude::*;

/// Interface for the blockchain node service implementation responsible for
/// validating and accepting transactions.
pub trait TransactionService {
    /// Transaction in the blockchain.
    type Transaction: Message + Serialize;

    /// The transaction identifier type for the blockchain.
    type TransactionId: MessageId + Serialize + Deserialize;

    /// The type of asynchronous futures returned by method `propose_transactions`.
    type ProposeTransactionsFuture: Future<
        Item = ProposeTransactionsResponse<Self::TransactionId>,
        Error = Error,
    >;

    /// The type of an asynchronous stream that provides block headers in
    /// response to `get_transactions`.
    type GetTransactionsStream: Stream<Item = Self::Transaction, Error = Error>;

    /// The type of asynchronous futures returned by `get_transactions`.
    ///
    /// The future resolves to a stream that will be used by the protocol
    /// implementation to produce a server-streamed response.
    type GetTransactionsFuture: Future<Item = Self::GetTransactionsStream, Error = Error>;

    /// The type of an asynchronous stream that provides transactions announced
    /// by the peer via the bidirectional subscription.
    type TransactionSubscription: Stream<Item = Self::Transaction, Error = Error>;

    /// The future resolves to a stream that will be used by the protocol
    /// implementation to produce a server-streamed response.
    type TransactionSubscriptionFuture: Future<Item = Self::TransactionSubscription, Error = Error>;

    /// Get all transactions by their id.
    fn get_transactions(&mut self, ids: &[Self::TransactionId]) -> Self::GetTransactionsFuture;

    /// Given a list of transaction IDs, return status of the transactions
    /// as known by this node.
    ///
    /// This method is only used by the NTT implementation.
    fn propose_transactions(
        &mut self,
        ids: &[Self::TransactionId],
    ) -> Self::ProposeTransactionsFuture;

    // Establishes a bidirectional subscription for announcing transactions,
    // taking an asynchronous stream that provides the inbound announcements.
    //
    // Returns a future that resolves to an asynchronous subscription stream
    // that receives transactions announced by the peer.
    fn transaction_subscription<In>(&mut self, inbound: In) -> Self::TransactionSubscriptionFuture
    where
        In: Stream<Item = Self::Transaction, Error = Error>;
}

/// Response from the `propose_transactions` method of a `TransactionService`.
pub struct ProposeTransactionsResponse<Id> {
    // TODO: define fully
    _ids: Vec<Id>,
}
