//! Transaction service abstraction.

use crate::codes;
use crate::error::Code as ErrorCode;

use chain_core::property::{Serialize, TransactionId};

use futures::prelude::*;

use std::{error, fmt};

/// Interface for the blockchain node service implementation responsible for
/// validating and accepting transactions.
pub trait TransactionService {
    /// The transaction identifier type for the blockchain.
    type TransactionId: TransactionId + Serialize;

    /// The type of asynchronous futures returned by method `propose_transactions`.
    type ProposeTransactionsFuture: Future<
        Item = ProposeTransactionsResponse<Self::TransactionId>,
        Error = TransactionError,
    >;

    /// The type of asynchronous futures returned by method `record_transaction`.
    type RecordTransactionFuture: Future<
        Item = RecordTransactionResponse<Self::TransactionId>,
        Error = TransactionError,
    >;

    /// Given a list of transaction IDs, return status of the transactions
    /// as known by this node.
    fn propose_transactions(
        &mut self,
        ids: &[Self::TransactionId],
    ) -> Self::ProposeTransactionsFuture;
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
    _items: Vec<(Id, codes::TransactionStatus)>,
}

/// Response from the `record_transactions` method of a `TransactionService`.
pub struct RecordTransactionResponse<Id> {
    // TODO: define
    _id: Id,
    _result: codes::TransactionAcceptance,
}
