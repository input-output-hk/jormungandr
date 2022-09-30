pub mod account;
pub mod committee;
pub mod delegation;
pub mod discrimination;
pub mod utxo;

use crate::{
    wallet::discrimination::DiscriminationExtension, FragmentBuilder, FragmentBuilderError,
};
use chain_addr::{AddressReadable, Discrimination};
use chain_crypto::{Ed25519, Ed25519Extended, PublicKey, SecretKey, Signature};
pub use chain_impl_mockchain::{
    account::SpendingCounter,
    block::Block,
    certificate::{PoolId, SignedCertificate, UpdateProposal, UpdateVote},
    chaintypes::ConsensusVersion,
    fee::LinearFee,
    fragment::Fragment,
    header::HeaderId,
    milli::Milli,
    transaction::{Input, TransactionBindingAuthData},
};
use chain_impl_mockchain::{
    accounting::account::SpendingCounterIncreasing,
    block::BlockDate,
    fee::FeeAlgorithm,
    key::EitherEd25519SecretKey,
    testing::data::{AddressData, AddressDataValue, Wallet as WalletLib},
    transaction::{
        InputOutputBuilder, Payload, PayloadSlice, TransactionBindingAuthDataPhantom,
        TransactionSignDataHash, UnspecifiedAccountIdentifier, Witness,
    },
    value::Value as ValueLib,
    vote::CommitteeId,
};
use jormungandr_automation::jcli::WitnessData;
use jormungandr_lib::{
    crypto::{account::Identifier as AccountIdentifier, hash::Hash, key::Identifier},
    interfaces::{Address, CommitteeIdDef, Destination, Initial, InitialUTxO, Value},
};
use rand_core::{CryptoRng, RngCore};
use std::{fs::File, path::Path};
use thiserror::Error;

pub type WalletAlias = String;

#[derive(Error, Debug)]
pub enum WalletError {
    #[error("couldn't create file")]
    IoError(#[from] std::io::Error),
    #[error("cannot add input to the transaction")]
    CannotAddInput,
    #[error("cannot make witness for the transaction")]
    CannotMakeWitness,
    #[error("transaction error")]
    FragmentError(#[from] FragmentBuilderError),
    #[error("Invalid data")]
    InvalidBech32(#[from] bech32::Error),
    #[error("invalid electin public key")]
    ElectionPublicKey,
    #[error("invalid bech32 public key, expected {expected} hrp got {actual}")]
    InvalidBech32Key { expected: String, actual: String },
}

const DEFAULT_LANE: usize = 0;

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone)]
pub enum Wallet {
    Account(account::Wallet),
    UTxO(utxo::Wallet),
    Delegation(delegation::Wallet),
}

impl Default for Wallet {
    fn default() -> Self {
        Self::new_account(&mut rand::rngs::OsRng, Discrimination::Test)
    }
}

impl Wallet {
    pub fn new_account<RNG>(rng: &mut RNG, discrimination: Discrimination) -> Wallet
    where
        RNG: CryptoRng + RngCore,
    {
        Self::new_account_with_discrimination(rng, discrimination)
    }

    pub fn import_account<P: AsRef<Path>>(
        secret_key_file: P,
        spending_counter: Option<SpendingCounter>,
        discrimination: Discrimination,
    ) -> Wallet {
        let bech32_str = jortestkit::file::read_file(secret_key_file).unwrap();
        Wallet::Account(account::Wallet::from_existing_account(
            &bech32_str,
            spending_counter.map(Into::into),
            discrimination,
        ))
    }

    pub fn new_account_with_discrimination<RNG>(
        rng: &mut RNG,
        discrimination: Discrimination,
    ) -> Wallet
    where
        RNG: CryptoRng + RngCore,
    {
        Wallet::Account(account::Wallet::generate(rng, discrimination))
    }

    pub fn from_existing_account(
        signing_key_bech32: &str,
        spending_counter: Option<SpendingCounter>,
        discrimination: Discrimination,
    ) -> Wallet {
        Wallet::Account(account::Wallet::from_existing_account(
            signing_key_bech32,
            spending_counter.map(Into::into),
            discrimination,
        ))
    }

    pub fn discrimination(&self) -> Discrimination {
        self.address().1 .0
    }

    pub fn to_initial_fund(&self, value: u64) -> InitialUTxO {
        InitialUTxO {
            address: self.address(),
            value: value.into(),
        }
    }

    pub fn to_initial_token(&self, value: u64) -> Destination {
        Destination {
            address: self.address(),
            value: value.into(),
        }
    }

    pub fn new_utxo<RNG>(rng: &mut RNG) -> Wallet
    where
        RNG: CryptoRng + RngCore,
    {
        Self::new_utxo_with_discrimination(rng, Discrimination::Test)
    }

    pub fn new_utxo_with_discrimination<RNG>(
        rng: &mut RNG,
        discrimination: Discrimination,
    ) -> Wallet
    where
        RNG: CryptoRng + RngCore,
    {
        Wallet::UTxO(utxo::Wallet::generate(rng, discrimination))
    }

    pub fn new_delegation<RNG>(delegation_identifier: &AccountIdentifier, rng: &mut RNG) -> Wallet
    where
        RNG: CryptoRng + RngCore,
    {
        Self::new_delegation_with_discrimination(delegation_identifier, rng, Discrimination::Test)
    }

    pub fn new_delegation_with_discrimination<RNG>(
        delegation_identifier: &AccountIdentifier,
        rng: &mut RNG,
        discrimination: Discrimination,
    ) -> Wallet
    where
        RNG: CryptoRng + RngCore,
    {
        let mut delegation = delegation::Wallet::generate(rng, discrimination);
        delegation.generate_new_signing_key(delegation_identifier.clone());
        Wallet::Delegation(delegation)
    }

    pub fn secret_key(&self) -> SecretKey<Ed25519Extended> {
        match self {
            Wallet::Account(account) => {
                let secret_key = match account.signing_key().as_ref() {
                    EitherEd25519SecretKey::Extended(secret_key) => secret_key,
                    EitherEd25519SecretKey::Normal(_) => panic!("unsupported secret key type"),
                };
                secret_key.clone()
            }
            Wallet::UTxO(utxo) => utxo.last_signing_key().clone().into_secret_key(),
            Wallet::Delegation(delegation) => {
                delegation.last_signing_key().clone().into_secret_key()
            }
        }
    }

    pub fn save_to_path<P: AsRef<Path>>(&self, path: P) -> std::io::Result<()> {
        let file = File::create(path).unwrap();
        self.save_to(&file)
    }

    pub fn save_to<W: std::io::Write>(&self, w: W) -> std::io::Result<()> {
        match self {
            Wallet::Account(account) => account.save_to(w),
            Wallet::UTxO(utxo) => utxo.save_to(w),
            _ => unimplemented!(),
        }
    }

    pub fn address(&self) -> Address {
        match self {
            Wallet::Account(account) => account.address(),
            Wallet::UTxO(utxo) => utxo.address(),
            Wallet::Delegation(delegation) => delegation.address(),
        }
    }

    pub fn public_key(&self) -> PublicKey<Ed25519> {
        self.address().1.public_key().unwrap().clone()
    }

    pub fn public_key_bech32(&self) -> String {
        hex::encode(Identifier::from(self.public_key()).as_ref())
    }

    pub fn address_bech32(&self, discrimination: Discrimination) -> String {
        AddressReadable::from_address(&discrimination.into_prefix(), &self.address().into())
            .to_string()
    }

    pub fn sign_slice(&self, data: &[u8]) -> Signature<TransactionBindingAuthDataPhantom, Ed25519> {
        match self {
            Wallet::Account(account) => account.signing_key().as_ref().sign_slice(data),
            Wallet::UTxO(utxo) => utxo.last_signing_key().as_ref().sign_slice(data),
            Wallet::Delegation(delegation) => {
                delegation.last_signing_key().as_ref().sign_slice(data)
            }
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

    pub fn account_id(&self) -> AccountIdentifier {
        match self {
            Wallet::Account(account) => account.identifier().as_ref().clone().into(),
            Wallet::UTxO(_utxo) => unimplemented!(),
            Wallet::Delegation(_delegation) => unimplemented!(),
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

    pub fn add_input_with_value(&self, value: Value) -> Input {
        match self {
            Wallet::Account(account) => account.add_input_with_value(value),
            Wallet::UTxO(_utxo) => unimplemented!(),
            Wallet::Delegation(_delegation) => unimplemented!(),
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
        self.confirm_transaction_at_lane(DEFAULT_LANE)
    }

    pub fn confirm_transaction_at_lane(&mut self, lane: usize) {
        match self {
            Wallet::Account(account) => account.increment_counter(lane),
            _ => unimplemented!(),
        }
    }

    pub fn decrement_counter(&mut self) {
        match self {
            Wallet::Account(account) => account.decrement_counter(DEFAULT_LANE),
            _ => unimplemented!(),
        }
    }

    pub fn spending_counter(&self) -> Option<SpendingCounterIncreasing> {
        match self {
            Wallet::Account(account) => {
                SpendingCounterIncreasing::new_from_counters(account.internal_counters())
            }
            _ => unimplemented!(),
        }
    }

    pub fn stake_key(&self) -> Option<UnspecifiedAccountIdentifier> {
        match &self {
            Wallet::Account(account) => Some(account.stake_key()),
            Wallet::Delegation(delegation) => Some(delegation.stake_key()),
            _ => unimplemented!(),
        }
    }

    pub fn delegation_cert_for_block0(&self, valid_until: BlockDate, pool_id: PoolId) -> Initial {
        FragmentBuilder::full_delegation_cert_for_block0(valid_until, self, pool_id)
    }

    pub fn to_committee_id(&self) -> CommitteeIdDef {
        CommitteeIdDef::from(CommitteeId::from(
            self.address().1.public_key().unwrap().clone(),
        ))
    }

    pub fn update_counter(&mut self, counter: SpendingCounter) {
        if let Wallet::Account(account) = self {
            account.set_counter(counter)
        }
    }

    pub fn witness_data(&self) -> WitnessData {
        match self {
            Self::Account(account) => WitnessData::new_account(
                &account.signing_key().to_bech32_str(),
                account.internal_counter(),
            ),
            Self::UTxO(utxo) => WitnessData::new_utxo(&utxo.last_signing_key().to_bech32_str()),
            Self::Delegation(delegation) => {
                WitnessData::new_utxo(&delegation.last_signing_key().to_bech32_str())
            }
        }
    }
}

impl From<Wallet> for WalletLib {
    fn from(wallet: Wallet) -> WalletLib {
        let address_data = match wallet {
            Wallet::Account(account) => AddressData::new(
                account.signing_key().as_ref().clone(),
                account.spending_counter().clone(),
                account.address().into(),
            ),
            Wallet::UTxO(utxo) => AddressData::new(
                EitherEd25519SecretKey::Extended(utxo.last_signing_key().as_ref().clone()),
                Default::default(),
                utxo.address().into(),
            ),
            Wallet::Delegation(delegation) => AddressData::new(
                EitherEd25519SecretKey::Extended(delegation.last_signing_key().as_ref().clone()),
                Default::default(),
                delegation.address().into(),
            ),
        };
        let address_data_value = AddressDataValue::new(address_data, ValueLib(0));
        WalletLib::from_address_data_value(address_data_value)
    }
}

impl From<account::Wallet> for Wallet {
    fn from(account: account::Wallet) -> Self {
        Self::Account(account)
    }
}
