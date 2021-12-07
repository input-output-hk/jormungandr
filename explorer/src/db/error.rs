use chain_impl_mockchain::{
    block::{ChainLength, HeaderId as HeaderHash},
    fragment::FragmentId,
};
use thiserror::Error;

#[derive(Debug, Error, Clone)]
pub enum ExplorerError {
    #[error(transparent)]
    BlockNotFound(#[from] BlockNotFound),
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

#[derive(Debug, Error, Clone)]
#[error("block {hash} not found in explorer")]
pub struct BlockNotFound {
    pub hash: HeaderHash,
}
