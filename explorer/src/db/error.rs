use chain_impl_mockchain::{
    block::{ChainLength, HeaderId as HeaderHash},
    fragment::FragmentId,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ExplorerError {
    #[error("block {0} not found in explorer")]
    BlockNotFound(HeaderHash),
    #[error("ancestor of block '{0}' not found in explorer")]
    AncestorNotFound(HeaderHash),
    #[error("transaction '{0}' is already indexed")]
    TransactionAlreadyExists(FragmentId),
    #[error("tried to index block '{0}' twice")]
    BlockAlreadyExists(HeaderHash),
    #[error("block with {0} chain length already exists in explorer branch")]
    ChainLengthBlockAlreadyExists(ChainLength),
    #[error("the explorer's database couldn't be initialized: {0}")]
    BootstrapError(String),
}

pub type Result<T> = std::result::Result<T, ExplorerError>;
