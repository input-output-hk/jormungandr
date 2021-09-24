pub mod template;

pub use template::{ExternalWalletTemplate, WalletTemplate};

use crate::wallet::{
    account::Wallet as AccountWallet, utxo::Wallet as UtxOWallet, Wallet as Inner, WalletError,
};
use chain_impl_mockchain::{
    block::BlockDate, certificate::PoolId, fee::LinearFee, fragment::Fragment,
    transaction::UnspecifiedAccountIdentifier, vote::CommitteeId,
};
use jormungandr_lib::{
    crypto::hash::Hash,
    interfaces::{Address, Initial, Value},
};
use rand_core::{CryptoRng, RngCore};
use serde::Deserialize;
use std::path::Path;

pub type WalletAlias = String;

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum WalletType {
    Account,
    UTxO,
}

/// wallet to utilise when testing jormungandr
///
/// This can be used for a faucet
#[derive(Debug, Clone)]
pub struct Wallet {
    pub inner: Inner,
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
            inner: Inner::Account(AccountWallet::generate(rng, template.discrimination())),
            template,
        }
    }

    pub fn generate_utxo<RNG>(template: WalletTemplate, rng: &mut RNG) -> Self
    where
        RNG: CryptoRng + RngCore,
    {
        Wallet {
            inner: Inner::UTxO(UtxOWallet::generate(rng, template.discrimination())),
            template,
        }
    }

    pub fn address(&self) -> Address {
        self.inner.address()
    }

    pub fn committee_id(&self) -> CommitteeId {
        CommitteeId::from(self.address().1.public_key().unwrap().clone())
    }

    pub fn stake_key(&self) -> Option<UnspecifiedAccountIdentifier> {
        self.inner.stake_key()
    }

    pub fn delegation_cert_for_block0(&self, valid_until: BlockDate, pool_id: PoolId) -> Initial {
        self.inner.delegation_cert_for_block0(valid_until, pool_id)
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
        valid_until: BlockDate,
        address: Address,
        value: Value,
    ) -> Result<Fragment, WalletError> {
        self.inner
            .transaction_to(block0_hash, fees, valid_until, address, value)
    }
}

impl From<Wallet> for Inner {
    fn from(wallet: Wallet) -> Inner {
        wallet.inner
    }
}

pub type WalletLib = chain_impl_mockchain::testing::data::Wallet;

impl From<Wallet> for WalletLib {
    fn from(wallet: Wallet) -> WalletLib {
        let inner: Inner = wallet.into();
        inner.into()
    }
}
