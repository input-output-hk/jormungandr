mod certificate_builder;
mod fragment_check;
mod fragment_sender;
mod fragments_check;
mod transaction_builder;

pub use certificate_builder::CertificateBuilder;
pub use fragment_check::FragmentCheck;
pub use fragment_sender::FragmentSender;
pub use fragments_check::FragmentsCheck;
use jormungandr_lib::crypto::hash::Hash;
use thiserror::Error;
pub use transaction_builder::TransactionBuilder;

#[derive(Debug, Error)]
pub enum Error {
    #[error("transaction {transaction_id} is not in block. message log: {message_log}. Jormungandr log: {log_content}")]
    TransactionNotInBlock {
        message_log: String,
        transaction_id: Hash,
        log_content: String,
    },
    #[error("at least one transaction is not in block. message log: {message_log}. Jormungandr log: {log_content}")]
    TransactionsNotInBlock {
        message_log: String,
        log_content: String,
    },
}
