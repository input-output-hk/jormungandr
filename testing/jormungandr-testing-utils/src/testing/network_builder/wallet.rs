use super::NodeAlias;
use crate::wallet::{
    account::Wallet as AccountWallet, utxo::Wallet as UtxOWallet, Wallet as Inner, WalletError,
};
use chain_impl_mockchain::{
    certificate::PoolId, fee::LinearFee, fragment::Fragment,
    transaction::UnspecifiedAccountIdentifier,
};
use jormungandr_lib::{
    crypto::hash::Hash,
    interfaces::{Address, Initial, Value},
};
use rand_core::{CryptoRng, RngCore};
use std::path::Path;

pub type WalletAlias = String;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WalletType {
    Account,
    UTxO,
}

#[derive(Clone, Debug)]
pub struct WalletTemplate {
    alias: WalletAlias,
    value: Value,
    wallet_type: WalletType,
    delegate: Option<NodeAlias>,
}

impl WalletTemplate {
    pub fn new_account<S: Into<WalletAlias>>(alias: S, value: Value) -> Self {
        Self::new(alias, value, WalletType::Account)
    }
    pub fn new_utxo<S: Into<WalletAlias>>(alias: S, value: Value) -> Self {
        Self::new(alias, value, WalletType::UTxO)
    }

    #[inline]
    fn new<S: Into<WalletAlias>>(alias: S, value: Value, wallet_type: WalletType) -> Self {
        Self {
            alias: alias.into(),
            value,
            wallet_type,
            delegate: None,
        }
    }

    pub fn alias(&self) -> &WalletAlias {
        &self.alias
    }

    pub fn wallet_type(&self) -> &WalletType {
        &self.wallet_type
    }

    pub fn value(&self) -> &Value {
        &self.value
    }

    pub fn delegate(&self) -> &Option<NodeAlias> {
        &self.delegate
    }

    pub fn delegate_mut(&mut self) -> &mut Option<NodeAlias> {
        &mut self.delegate
    }
}

/// wallet to utilise when testing jormungandr
///
/// This can be used for a faucet
#[derive(Debug, Clone)]
pub struct Wallet {
    inner: Inner,
    template: WalletTemplate,
}

impl Wallet {
    pub fn save_to<P: AsRef<Path>>(&self, dir: P) -> std::io::Result<()> {
        let dir = dir.as_ref().join(self.template().alias());
        let file = std::fs::File::create(&dir)?;
        self.inner.save_to(file)
    }

    pub fn generate_account<RNG>(template: WalletTemplate, rng: &mut RNG) -> Self
    where
        RNG: CryptoRng + RngCore,
    {
        Wallet {
            inner: Inner::Account(AccountWallet::generate(rng)),
            template,
        }
    }

    pub fn generate_utxo<RNG>(template: WalletTemplate, rng: &mut RNG) -> Self
    where
        RNG: CryptoRng + RngCore,
    {
        Wallet {
            inner: Inner::UTxO(UtxOWallet::generate(rng)),
            template,
        }
    }

    pub fn address(&self) -> Address {
        self.inner.address()
    }

    pub fn stake_key(&self) -> Option<UnspecifiedAccountIdentifier> {
        self.inner.stake_key()
    }

    pub fn delegation_cert_for_block0(&self, pool_id: PoolId) -> Initial {
        self.inner.delegation_cert_for_block0(pool_id)
    }

    pub fn template(&self) -> &WalletTemplate {
        &self.template
    }

    pub fn confirm_transaction(&mut self) {
        self.inner.confirm_transaction()
    }

    pub fn identifier(&mut self) -> chain_impl_mockchain::account::Identifier {
        match &mut self.inner {
            Inner::Account(account) => account.identifier().to_inner(),
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
        self.inner.transaction_to(block0_hash, fees, address, value)
    }
}

impl Into<Inner> for Wallet {
    fn into(self) -> Inner {
        self.inner
    }
}
