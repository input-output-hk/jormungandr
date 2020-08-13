use crate::WalletBackend;
use crate::{Proposal, Wallet};
use bip39::Type;
use chain_crypto::{bech32::Bech32, Ed25519, PublicKey};
use chain_impl_mockchain::fragment::FragmentId;
use jormungandr_testing_utils::testing::node::RestSettings;
use std::iter;
use thiserror::Error;
use wallet::Settings;
use wallet_core::{Choice, Value};
pub struct MultiController {
    backend: WalletBackend,
    wallets: Vec<Wallet>,
    settings: Settings,
}

impl MultiController {
    pub fn generate(
        wallet_backend_address: String,
        words_length: Type,
        count: usize,
        backend_settings: RestSettings,
    ) -> Result<Self, MultiControllerError> {
        let backend = WalletBackend::new(wallet_backend_address, backend_settings);
        let settings = backend.settings()?;
        let wallets = iter::from_fn(|| Some(Wallet::generate(words_length).unwrap()))
            .take(count)
            .collect();
        Ok(Self {
            backend,
            wallets,
            settings,
        })
    }

    pub fn recover(
        wallet_backend_address: &str,
        mnemonics: Vec<String>,
        password: &[u8],
        backend_settings: RestSettings,
    ) -> Result<Self, MultiControllerError> {
        let mut backend = WalletBackend::new(wallet_backend_address.to_string(), backend_settings);
        let settings = backend.settings()?;
        let wallets = mnemonics
            .iter()
            .map(|x| Wallet::recover(x, password).unwrap())
            .collect();
        Ok(Self {
            backend,
            wallets,
            settings,
        })
    }

    pub fn retrieve_funds(&mut self) -> Result<(), MultiControllerError> {
        let block_bytes = self.backend.block0()?;
        for wallet in self.wallets.iter_mut() {
            wallet.retrieve_funds(&block_bytes)?;
        }
        Ok(())
    }

    pub fn retrieve_conversion_transactions(
        &mut self,
    ) -> Result<Vec<Vec<u8>>, MultiControllerError> {
        let mut output = Vec::new();
        let block0 = self.backend().block0()?;
        for wallet in self.wallets.iter_mut() {
            if self.backend.account_exists(wallet.id())? {
                continue;
            }
            wallet.retrieve_funds(&block0)?;
            for tx in wallet.convert(self.settings.clone()).transactions() {
                output.push(tx.clone());
            }
        }
        Ok(output)
    }

    pub fn proposals(&self) -> Result<Vec<Proposal>, MultiControllerError> {
        self.backend.proposals().map_err(Into::into)
    }

    pub(crate) fn backend(&self) -> &WalletBackend {
        &self.backend
    }

    pub fn vote(
        &mut self,
        wallet_index: usize,
        proposal: &Proposal,
        choice: Choice,
    ) -> Result<FragmentId, MultiControllerError> {
        let wallet = self.wallets.get_mut(wallet_index).unwrap();
        let tx = wallet.vote(self.settings.clone(), &proposal.clone().into(), choice)?;
        self.backend()
            .send_fragment(tx.to_vec())
            .map_err(Into::into)
    }

    pub fn confirm_all_transactions(&mut self) {
        for wallet in self.wallets.iter_mut() {
            wallet.confirm_all_transactions();
        }
    }

    pub fn confirm_transaction(&mut self, fragment_id: FragmentId) {
        for wallet in self.wallets.iter_mut() {
            wallet.confirm_transaction(fragment_id);
        }
    }

    pub fn refresh_wallet(&mut self, wallet_index: usize) -> Result<(), MultiControllerError> {
        let wallet = self.wallets.get_mut(wallet_index).unwrap();
        let account_state = self.backend.account_state(wallet.id())?;
        let value: u64 = (*account_state.value()).into();
        wallet.set_state(Value(value), account_state.counter());
        Ok(())
    }

    pub fn wallet_count(&self) -> usize {
        self.wallets.len()
    }

    pub fn is_converted(&mut self, wallet_index: usize) -> Result<bool, MultiControllerError> {
        let wallet = self.wallets.get_mut(wallet_index).unwrap();
        self.backend.account_exists(wallet.id()).map_err(Into::into)
    }
}

#[derive(Debug, Error)]
pub enum MultiControllerError {
    #[error("wallet error")]
    WalletError(#[from] crate::wallet::WalletError),
    #[error("wallet error")]
    BackendError(#[from] crate::backend::WalletBackendError),
}
