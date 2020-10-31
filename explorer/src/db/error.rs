use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("block `{0}` cannot be found in the explorer")]
    BlockNotFound(String),
    #[error("ancestor of block `{0}` cannot be found in the explorer")]
    AncestorNotFound(String),
    #[error("transaction `{0}` is already indexed")]
    TransactionAlreadyExists(String),
    #[error("block `{0}` is already indexed")]
    BlockAlreadyExists(String),
    #[error("chain length: `{0}` is already indexed")]
    ChainLengthBlockAlreadyExists(u32),
    #[error("bootstrap error: `{0}`")]
    BootstrapError(String),
}
