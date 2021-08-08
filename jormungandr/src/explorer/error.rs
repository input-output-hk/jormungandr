use crate::blockcfg::HeaderHash;
use crate::{blockchain::Error as ChainError, blockchain::StorageError, intercom};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ExplorerError {
    #[error("block {0} not found in explorer")]
    BlockNotFound(HeaderHash),
    #[error("ancestor of block '{0}' not found in explorer")]
    AncestorNotFound(HeaderHash),
    #[error("transaction '{0}' is already indexed")]
    TransactionAlreadyExists(crate::blockcfg::FragmentId),
    #[error("tried to index block '{0}' twice")]
    BlockAlreadyExists(HeaderHash),
    #[error("block with {0} chain length already exists in explorer branch")]
    ChainLengthBlockAlreadyExists(crate::blockcfg::ChainLength),
    #[error("the explorer's database couldn't be initialized: {0}")]
    BootstrapError(String),
    #[error("storage error")]
    StorageError(#[from] StorageError),
    #[error("blockchain error")]
    ChainError(#[from] Box<ChainError>),
    #[error("streaming error")]
    StreamingError(#[from] intercom::Error),
}

pub type Result<T> = std::result::Result<T, ExplorerError>;
