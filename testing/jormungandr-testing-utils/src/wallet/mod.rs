pub mod account;
pub mod delegation;
pub mod utxo;

use crate::{
    stake_pool::StakePool,
    testing::{FragmentBuilder, FragmentBuilderError},
};
use chain_impl_mockchain::{
    fee::FeeAlgorithm,
    key::EitherEd25519SecretKey,
    testing::data::{AddressData, AddressDataValue, Wallet as WalletLib},
    transaction::{
        InputOutputBuilder, Payload, PayloadSlice, TransactionBindingAuthDataPhantom,
        TransactionSignDataHash, Witness,
    },
    value::Value as ValueLib,
};
use jormungandr_lib::{
    crypto::{account::Identifier as AccountIdentifier, hash::Hash, key::Identifier},
    interfaces::{Address, Initial, Value},
};

use chain_addr::Discrimination;
use chain_crypto::{Ed25519, Signature};
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
    transaction::{TransactionBindingAuthData, UnspecifiedAccountIdentifier},
};

#[derive(Error, Debug)]
pub enum WalletError {
    #[error("couldn't create file")]
    IOError(#[from] std::io::Error),
    #[error("cannot add input to the transaction")]
    CannotAddInput,
    #[error("cannot make witness for the transaction")]
    CannotMakeWitness,
    #[error("transaction error")]
    FragmentError(#[from] FragmentBuilderError),
}

#[allow(clippy::large_enum_variant)]
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

    pub fn sign_slice(&self, data: &[u8]) -> Signature<TransactionBindingAuthDataPhantom, Ed25519> {
        match self {
            Wallet::Account(account) => account.signing_key().as_ref().sign_slice(&data),
            _ => unimplemented!(),
        }
    }

    /// Temporary method exposing private key
    pub fn signing_key_to_string(&self) -> String {
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

    pub fn add_input<'a, Extra: Payload>(
        &self,
        payload: PayloadSlice<'a, Extra>,
        iobuilder: &mut InputOutputBuilder,
        fees: &LinearFee,
    ) -> Result<(), FragmentBuilderError>
    where
        LinearFee: FeeAlgorithm,
    {
        match self {
            Wallet::Account(account) => account.add_input(payload, iobuilder, fees),
            Wallet::UTxO(_utxo) => unimplemented!(),
            Wallet::Delegation(_delegation) => unimplemented!(),
        }
    }

    pub fn mk_witness(
        &self,
        block0_hash: &Hash,
        signing_data: &TransactionSignDataHash,
    ) -> Witness {
        match self {
            Wallet::Account(account) => account.mk_witness(block0_hash, signing_data),
            Wallet::UTxO(utxo) => utxo.mk_witness(block0_hash, signing_data),
            Wallet::Delegation(delegation) => delegation.mk_witness(block0_hash, signing_data),
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

    pub fn delegation_cert_for_block0(&self, pool_id: PoolId) -> Initial {
        FragmentBuilder::full_delegation_cert_for_block0(&self, pool_id)
    }

    pub fn transaction_to(
        &mut self,
        block0_hash: &Hash,
        fees: &LinearFee,
        address: Address,
        value: Value,
    ) -> Result<Fragment, WalletError> {
        FragmentBuilder::new(block0_hash, fees)
            .transaction(&self, address, value)
            .map_err(WalletError::FragmentError)
    }

    pub fn issue_pool_retire_cert(
        &mut self,
        block0_hash: &Hash,
        fees: &LinearFee,
        stake_pool: &StakePool,
    ) -> Result<Fragment, WalletError> {
        Ok(FragmentBuilder::new(block0_hash, fees).stake_pool_retire(vec![&self], stake_pool))
    }

    pub fn issue_pool_registration_cert(
        &mut self,
        block0_hash: &Hash,
        fees: &LinearFee,
        stake_pool: &StakePool,
    ) -> Result<Fragment, WalletError> {
        Ok(FragmentBuilder::new(block0_hash, fees).stake_pool_registration(&self, stake_pool))
    }

    pub fn issue_pool_update_cert(
        &mut self,
        block0_hash: &Hash,
        fees: &LinearFee,
        stake_pool: &StakePool,
        update_stake_pool: &StakePool,
    ) -> Result<Fragment, WalletError> {
        Ok(FragmentBuilder::new(block0_hash, fees).stake_pool_update(
            vec![&self],
            stake_pool,
            update_stake_pool,
        ))
    }

    pub fn issue_full_delegation_cert(
        &mut self,
        block0_hash: &Hash,
        fees: &LinearFee,
        stake_pool: &StakePool,
    ) -> Result<Fragment, WalletError> {
        Ok(FragmentBuilder::new(block0_hash, fees).delegation(&self, stake_pool))
    }

    pub fn issue_owner_delegation_cert(
        &mut self,
        block0_hash: &Hash,
        fees: &LinearFee,
        stake_pool: &StakePool,
    ) -> Result<Fragment, WalletError> {
        Ok(FragmentBuilder::new(block0_hash, fees).owner_delegation(&self, stake_pool))
    }

    pub fn issue_split_delegation_cert(
        &mut self,
        block0_hash: &Hash,
        fees: &LinearFee,
        distribution: Vec<(&StakePool, u8)>,
    ) -> Result<Fragment, WalletError> {
        Ok(FragmentBuilder::new(block0_hash, fees).delegation_to_many(&self, distribution))
    }

    pub fn remove_delegation_cert(
        &mut self,
        block0_hash: &Hash,
        fees: &LinearFee,
    ) -> Result<Fragment, WalletError> {
        Ok(FragmentBuilder::new(block0_hash, fees).delegation_remove(&self))
    }
}

impl Into<WalletLib> for Wallet {
    fn into(self) -> WalletLib {
        let address_data = match self {
            Wallet::Account(account) => AddressData::new(
                account.signing_key().as_ref().clone(),
                Some(account.internal_counter()),
                account.address(Discrimination::Test).into(),
            ),
            Wallet::UTxO(utxo) => AddressData::new(
                EitherEd25519SecretKey::Normal(utxo.last_signing_key().as_ref().clone()),
                None,
                utxo.address(Discrimination::Test).into(),
            ),
            Wallet::Delegation(delegation) => AddressData::new(
                EitherEd25519SecretKey::Normal(delegation.last_signing_key().as_ref().clone()),
                None,
                delegation.address(Discrimination::Test).into(),
            ),
        };
        let address_data_value = AddressDataValue::new(address_data, ValueLib(0));
        WalletLib::from_address_data_value(address_data_value)
    }
}
