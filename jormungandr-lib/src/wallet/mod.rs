pub mod account;
pub mod delegation;
pub mod utxo;

use crate::{
    crypto::{account::Identifier as AccountIdentifier, hash::Hash, key::Identifier},
    interfaces::{Address, Value},
};
use chain_addr::Discrimination;
use chain_crypto::Ed25519;

use rand_core::{CryptoRng, RngCore};
use thiserror::Error;

pub use chain_impl_mockchain::{
    block::Block,
    certificate::{PoolId, SignedCertificate},
    chaintypes::ConsensusVersion,
    fee::LinearFee,
    fragment::Fragment,
    header::HeaderId,
    milli::Milli,
    transaction::UnspecifiedAccountIdentifier,
};

#[derive(Error, Debug)]
pub enum WalletError {
    #[error("couldn't create file")]
    IOError(#[from] std::io::Error),
    #[error("cannot add input to the transaction")]
    CannotAddInput,
    #[error("cannot make witness for the transaction")]
    CannotMakeWitness,
    #[error("cannot compute the transaction's balance")]
    CannotComputeBalance,
    #[error("Cannot compute the new fees of {0} for a new input")]
    CannotAddCostOfExtraInput(u64),
    #[error("transaction already balanced")]
    TransactionAlreadyBalanced,
    #[error("the transaction has {0} value extra than necessary")]
    TransactionAlreadyExtraValue(Value),
}

#[derive(Debug, Clone)]
pub enum Wallet {
    Account(account::Wallet),
    UTxO(utxo::Wallet),
    Delegation(delegation::Wallet),
}

impl Wallet {
    pub fn new_account<RNG>(rng: &mut RNG) -> Wallet
    where
        RNG: CryptoRng + RngCore,
    {
        Wallet::Account(account::Wallet::generate(rng))
    }

    pub fn from_existing_account(
        signing_key_bech32: &str,
        spending_counter: Option<u32>,
    ) -> Wallet {
        Wallet::Account(account::Wallet::from_existing_account(
            signing_key_bech32,
            spending_counter,
        ))
    }

    pub fn new_utxo<RNG>(rng: &mut RNG) -> Wallet
    where
        RNG: CryptoRng + RngCore,
    {
        Wallet::UTxO(utxo::Wallet::generate(rng))
    }

    pub fn new_delegation<RNG>(delegation_identifier: &AccountIdentifier, rng: &mut RNG) -> Wallet
    where
        RNG: CryptoRng + RngCore,
    {
        let mut delegation = delegation::Wallet::generate(rng);
        delegation.generate_new_signing_key(delegation_identifier.clone());
        Wallet::Delegation(delegation)
    }

    pub fn address(&self) -> Address {
        match self {
            Wallet::Account(account) => account.address(Discrimination::Test),
            Wallet::UTxO(utxo) => utxo.address(Discrimination::Test),
            Wallet::Delegation(delegation) => delegation.address(Discrimination::Test),
        }
    }

    /// Temporary method exposing private key
    pub fn signing_key_as_str(&self) -> String {
        match self {
            Wallet::Account(account) => account.signing_key().to_bech32_str(),
            Wallet::UTxO(utxo) => utxo.last_signing_key().to_bech32_str(),
            Wallet::Delegation(delegation) => delegation.last_signing_key().to_bech32_str(),
        }
    }

    pub fn identifier(&self) -> Identifier<Ed25519> {
        match self {
            Wallet::Account(account) => Identifier::from(account.identifier().as_ref().clone()),
            Wallet::UTxO(utxo) => utxo.identifier(),
            Wallet::Delegation(delegation) => delegation.identifier(),
        }
    }

    pub fn delegation_key(&self) -> Identifier<Ed25519> {
        match self {
            Wallet::Delegation(delegation) => {
                Identifier::from(delegation.last_delegation_identifier().as_ref().clone())
            }
            _ => unimplemented!(),
        }
    }

    pub fn confirm_transaction(&mut self) {
        match self {
            Wallet::Account(account) => account.increment_counter(),
            _ => unimplemented!(),
        }
    }

    pub fn stake_key(&self) -> Option<UnspecifiedAccountIdentifier> {
        match &self {
            Wallet::Account(account) => Some(account.stake_key()),
            _ => unimplemented!(),
        }
    }

    pub fn delegation_cert_for_block0(&self, pool_id: PoolId) -> SignedCertificate {
        match &self {
            Wallet::Account(account) => account.delegation_cert_for_block0(pool_id),
            _ => unimplemented!(),
        }
    }

    pub fn transaction_to(
        &mut self,
        block0_hash: &Hash,
        fees: &LinearFee,
        address: Address,
        value: Value,
    ) -> Result<Fragment, WalletError> {
        match self {
            Wallet::Account(account) => account.transaction_to(block0_hash, fees, address, value),
            _ => unimplemented!(),
        }
    }
}
