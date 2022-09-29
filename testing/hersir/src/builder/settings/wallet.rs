use crate::config::WalletTemplate;
use chain_impl_mockchain::{
    block::BlockDate, certificate::PoolId, transaction::UnspecifiedAccountIdentifier,
    vote::CommitteeId,
};
use jormungandr_lib::interfaces::{Address, Initial, InitialUTxO};
use rand_core::{CryptoRng, OsRng, RngCore};
use serde::Deserialize;
use std::{path::Path, str::FromStr};
use thor::{AccountWallet, UTxOWallet, Wallet as Inner, WalletAlias};

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
    inner: Option<Inner>,
    template: WalletTemplate,
}

impl Wallet {
    pub fn save_to<P: AsRef<Path>>(&self, dir: P) -> std::io::Result<()> {
        if let Some(inner) = &self.inner {
            let dir = dir.as_ref().join(self.template().id());
            let file = std::fs::File::create(&dir)?;
            inner.save_to(file)
        } else {
            Ok(())
        }
    }

    pub fn external(template: WalletTemplate) -> Self {
        Self {
            inner: None,
            template,
        }
    }

    pub fn generate_account<RNG>(template: WalletTemplate, rng: &mut RNG) -> Self
    where
        RNG: CryptoRng + RngCore,
    {
        Wallet {
            inner: Some(Inner::Account(AccountWallet::generate(
                rng,
                template.discrimination(),
            ))),
            template,
        }
    }

    pub fn generate_utxo<RNG>(template: WalletTemplate, rng: &mut RNG) -> Self
    where
        RNG: CryptoRng + RngCore,
    {
        Wallet {
            inner: Some(Inner::UTxO(UTxOWallet::generate(
                rng,
                template.discrimination(),
            ))),
            template,
        }
    }

    pub fn has_alias(&self, alias: &WalletAlias) -> bool {
        self.template().has_alias(alias)
    }

    pub fn address(&self) -> Result<Address, Error> {
        if let Some(inner) = &self.inner {
            Ok(inner.address())
        } else {
            Address::from_str(
                &self
                    .template
                    .address()
                    .ok_or(Error::CannotRetrieveAddress)?,
            )
            .map_err(|_| Error::CannotRetrieveAddress)
        }
    }

    pub fn committee_id(&self) -> Result<CommitteeId, Error> {
        Ok(CommitteeId::from(
            self.address()?.1.public_key().unwrap().clone(),
        ))
    }

    pub fn stake_key(&self) -> Option<UnspecifiedAccountIdentifier> {
        if let Some(inner) = &self.inner {
            inner.stake_key()
        } else {
            None
        }
    }

    pub fn delegation_cert_for_block0(
        &self,
        valid_until: BlockDate,
        pool_id: PoolId,
    ) -> Result<Initial, Error> {
        if let Some(inner) = &self.inner {
            Ok(inner.delegation_cert_for_block0(valid_until, pool_id))
        } else {
            Err(Error::OperationUnavailableForExternalWallet(
                "delegation_cert_for_block0".to_string(),
            ))
        }
    }

    pub fn template(&self) -> &WalletTemplate {
        &self.template
    }

    pub fn identifier(&self) -> chain_impl_mockchain::account::Identifier {
        match self.inner.as_ref().unwrap() {
            Inner::Account(account) => account.identifier().to_inner(),
            _ => unimplemented!(),
        }
    }

    pub fn to_initial_fund(&self) -> Result<InitialUTxO, Error> {
        Ok(InitialUTxO {
            address: self.address()?,
            value: (*self.template.value()).into(),
        })
    }
    pub fn inner(&self) -> &Option<Inner> {
        &self.inner
    }
}

impl From<Wallet> for Inner {
    fn from(wallet: Wallet) -> Inner {
        wallet
            .inner
            .unwrap_or_else(|| panic!("cannot convert into Inner wallet.. this is external wallet"))
    }
}

pub type WalletLib = chain_impl_mockchain::testing::data::Wallet;

impl TryFrom<Wallet> for WalletLib {
    type Error = Error;

    fn try_from(wallet: Wallet) -> Result<WalletLib, Error> {
        Ok(Inner::try_from(wallet)
            .map_err(|_| {
                Error::OperationUnavailableForExternalWallet("into WalletLib".to_string())
            })?
            .into())
    }
}

impl From<WalletTemplate> for Wallet {
    fn from(template: WalletTemplate) -> Self {
        let mut rng = OsRng;

        if let Some(_alias) = template.alias() {
            match template.wallet_type().unwrap() {
                WalletType::UTxO => Self::generate_utxo(template, &mut rng),
                WalletType::Account => Self::generate_account(template, &mut rng),
            }
        } else {
            Self::external(template)
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("cannot retrieve address probably wallet template was badly initialized")]
    CannotRetrieveAddress,
    #[error("cannot invoke operation '{0}' as it's not supported for externally created wallet")]
    OperationUnavailableForExternalWallet(String),
}
