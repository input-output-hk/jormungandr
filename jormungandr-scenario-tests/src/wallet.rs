use crate::scenario::Wallet as WalletTemplate;
use chain_addr::Discrimination;
use chain_impl_mockchain::{
    certificate::{PoolId, SignedCertificate, StakeDelegation},
    fee::{FeeAlgorithm, LinearFee},
    fragment::Fragment,
    transaction::{
        AccountBindingSignature, Balance, Input, InputOutputBuilder, NoExtra, Payload,
        PayloadSlice, TxBuilder, UnspecifiedAccountIdentifier,
    },
};
use jormungandr_lib::{
    crypto::hash::Hash,
    interfaces::{Address, Value},
    wallet::{
        account::Wallet as AccountWallet, utxo::Wallet as UtxOWallet, Wallet as Inner, WalletError,
    },
};
use rand_core::{CryptoRng, RngCore};
use std::path::Path;

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

        match &self.inner {
            Inner::Account(account) => account.save_to(file),
            _ => unimplemented!(),
        }
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

    pub fn address(&self, discrimination: Discrimination) -> Address {
        self.inner.address()
    }

    pub fn stake_key(&self) -> Option<UnspecifiedAccountIdentifier> {
        self.inner.stake_key()
    }

    pub fn delegation_cert_for_block0(&self, pool_id: PoolId) -> SignedCertificate {
        self.inner.delegation_cert_for_block0(pool_id)
    }

    pub(crate) fn template(&self) -> &WalletTemplate {
        &self.template
    }

    pub fn confirm_transaction(&mut self) {
        self.inner.confirm_transaction()
    }

    pub fn identifier(&mut self) -> chain_impl_mockchain::account::Identifier {
        match &mut self.inner {
            Inner::Account(account) => account.identifier().to_inner().into(),
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
