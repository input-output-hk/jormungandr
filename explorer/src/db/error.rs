use chain_impl_mockchain::block::HeaderId as HeaderHash;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DbError {
    #[error("ancestor of block '{0}' ('{1}') not found in explorer")]
    AncestorNotFound(HeaderHash, HeaderHash),
    #[error("tried to index block '{0}' twice")]
    BlockAlreadyExists(HeaderHash),
    #[error(transparent)]
    SanakirjaError(#[from] ::sanakirja::Error),
    #[error("the database was not initialized or was corrupted")]
    UnitializedDatabase,
}
