use crate::context::Context;
use chain_crypto::PublicKey;
use chain_impl_mockchain::account::AccountAlg;
use std::str::FromStr;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    ContextError(#[from] crate::context::Error),
    #[error("Can not parse address: {0}")]
    AddressParseError(String),
}

pub async fn get_jor_address(context: &Context, evm_id_hex: &str) -> Result<String, Error> {
    Ok(context
        .blockchain_tip()?
        .get_ref()
        .await
        .ledger()
        .evm_jormungandr_mapped_address(
            &chain_evm::Address::from_str(evm_id_hex)
                .map_err(|e| Error::AddressParseError(e.to_string()))?,
        )
        .to_string())
}

pub async fn get_evm_address(context: &Context, jor_id_hex: &str) -> Result<Option<String>, Error> {
    Ok(context
        .blockchain_tip()?
        .get_ref()
        .await
        .ledger()
        .evm_evm_mapped_address(
            &PublicKey::<AccountAlg>::from_str(jor_id_hex)
                .map_err(|e| Error::AddressParseError(e.to_string()))?
                .into(),
        )
        .map(|val| val.to_string()))
}
