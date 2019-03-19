//! Transaction service abstraction.

use crate::error::Code as ErrorCode;

use chain_core::property::{Deserialize, Serialize, Transaction, TransactionId};

use futures::prelude::*;

use std::{error, fmt};

/// Interface for the blockchain node service implementation responsible for
/// validating and accepting transactions.
pub trait TransactionService {
    /// Transaction in the blockchain.
    type Transaction: Transaction + Serialize;

    /// The transaction identifier type for the blockchain.
    type TransactionId: TransactionId + Serialize + Deserialize;

    /// The type of asynchronous futures returned by method `propose_transactions`.
    type ProposeTransactionsFuture: Future<
        Item = ProposeTransactionsResponse<Self::TransactionId>,
        Error = TransactionError,
    >;

    /// The type of an asynchronous stream that provides block headers in
    /// response to `get_transactions`.
    type GetTransactionsStream: Stream<Item = Self::Transaction, Error = TransactionError>;

    /// The type of asynchronous futures returned by `get_transactions`.
    ///
    /// The future resolves to a stream that will be used by the protocol
    /// implementation to produce a server-streamed response.
    type GetTransactionsFuture: Future<Item = Self::GetTransactionsStream, Error = TransactionError>;

    /// The type of asynchronous futures returned by `announce_transaction`.
    type AnnounceTransactionFuture: Future<Item = (), Error = TransactionError>;

    /// Get all transactions by their id.
    fn get_transactions(&mut self, ids: &[Self::TransactionId]) -> Self::GetTransactionsFuture;

    /// Given a list of transaction IDs, return status of the transactions
    /// as known by this node.
    fn propose_transactions(
        &mut self,
        ids: &[Self::TransactionId],
    ) -> Self::ProposeTransactionsFuture;

    fn announce_transaction(
        &mut self,
        id: &[Self::TransactionId],
    ) -> Self::AnnounceTransactionFuture;
}

/// Represents errors that can be returned by the transaction service.
#[derive(Debug)]
pub struct TransactionError {
    code: ErrorCode,
    cause: Option<Box<dyn error::Error + Send + Sync>>,
}

impl TransactionError {
    pub fn failed<E>(cause: E) -> Self
    where
        E: Into<Box<dyn error::Error + Send + Sync>>,
    {
        TransactionError {
            code: ErrorCode::Failed,
            cause: Some(cause.into()),
        }
    }

    pub fn with_code_and_cause<E>(code: ErrorCode, cause: E) -> Self
    where
        E: Into<Box<dyn error::Error + Send + Sync>>,
    {
        TransactionError {
            code,
            cause: Some(cause.into()),
        }
    }

    pub fn code(&self) -> ErrorCode {
        self.code
    }
}

impl error::Error for TransactionError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        if let Some(err) = &self.cause {
            Some(&**err)
        } else {
            None
        }
    }
}

impl fmt::Display for TransactionError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "transaction service error: {}", self.code)
    }
}

/// Response from the `propose_transactions` method of a `TransactionService`.
pub struct ProposeTransactionsResponse<Id> {
    // TODO: define fully
    _ids: Vec<Id>,
}
