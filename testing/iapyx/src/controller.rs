use crate::SimpleVoteStatus;
use crate::Wallet;
use crate::{data::Proposal as VitProposal, WalletBackend};
use bip39::Type;
use chain_impl_mockchain::{fragment::FragmentId, transaction::Input};
use jormungandr_lib::interfaces::{AccountState, FragmentLog, FragmentStatus};
use std::collections::HashMap;
use thiserror::Error;
use wallet::{AccountId, Settings};
use wallet_core::{Choice, Conversion, Proposal, Value};

pub struct Controller {
    backend: WalletBackend,
    wallet: Wallet,
    settings: Settings,
}

impl Controller {
    pub fn generate(proxy_address: String, words_length: Type) -> Result<Self, ControllerError> {
        let backend = WalletBackend::new(proxy_address);
        let settings = backend.settings()?;
        Ok(Self {
            backend,
            wallet: Wallet::generate(words_length)?,
            settings,
        })
    }

    pub fn recover(
        proxy_address: String,
        mnemonics: &str,
        password: &[u8],
    ) -> Result<Self, ControllerError> {
        let backend = WalletBackend::new(proxy_address);
        let settings = backend.settings()?;
        Ok(Self {
            backend,
            wallet: Wallet::recover(mnemonics, password)?,
            settings,
        })
    }

    pub fn switch_backend(&mut self, proxy_address: String) {
        self.backend = WalletBackend::new(proxy_address);
    }

    pub fn account(&self, discrimination: chain_addr::Discrimination) -> chain_addr::Address {
        self.wallet.account(discrimination)
    }

    pub fn id(&self) -> AccountId {
        self.wallet.id()
    }

    pub fn retrieve_funds(&mut self) -> Result<(), ControllerError> {
        let block0_bytes = self.backend.block0()?;
        self.wallet.retrieve_funds(&block0_bytes)?;
        Ok(())
    }

    pub fn convert(&mut self) -> Conversion {
        self.wallet.convert(self.settings.clone())
    }

    pub fn convert_and_send(&mut self) -> Result<(), ControllerError> {
        for transaction in self.convert().transactions() {
            self.backend.send_fragment(transaction.clone())?;
        }
        Ok(())
    }

    pub fn send_fragment(&self, transaction: &[u8]) -> Result<FragmentId, ControllerError> {
        self.backend
            .send_fragment(transaction.to_vec())
            .map_err(Into::into)
    }

    pub fn confirm_all_transactions(&mut self) {
        self.wallet.confirm_all_transactions();
    }

    pub fn confirm_transaction(&mut self, id: FragmentId) {
        self.wallet.confirm_transaction(id)
    }

    pub fn pending_transactions(&self) -> &HashMap<FragmentId, Vec<Input>> {
        &self.wallet.pending_transactions()
    }

    pub fn wait_for_pending_transactions(
        &mut self,
        pace: std::time::Duration,
    ) -> Result<(), ControllerError> {
        let mut limit = 60;
        loop {
            let ids: Vec<FragmentId> = self.pending_transactions().keys().cloned().collect();

            if limit <= 0 {
                return Err(ControllerError::TransactionsWerePendingForTooLong {
                    fragments: ids.clone(),
                });
            }

            if ids.len() == 0 {
                return Ok(());
            }

            let fragment_logs = self.backend.fragment_logs().unwrap();
            for id in ids.iter() {
                if let Some(fragment) = fragment_logs.get(id) {
                    match fragment.status() {
                        FragmentStatus::Rejected { .. } => {
                            self.remove_pending_transaction(id);
                        }
                        FragmentStatus::InABlock { .. } => {
                            self.confirm_transaction(*id);
                        }
                        _ => (),
                    };
                }
            }

            if ids.len() == 0 {
                return Ok(());
            } else {
                std::thread::sleep(pace);
                limit = limit + 1;
            }
        }
    }

    pub fn remove_pending_transaction(&mut self, id: &FragmentId) -> Option<Vec<Input>> {
        self.wallet.remove_pending_transaction(id)
    }

    pub fn total_value(&self) -> Value {
        self.wallet.total_value()
    }

    pub fn refresh_state(&mut self) -> Result<(), ControllerError> {
        let account_state = self.get_account_state()?;
        let value: u64 = (*account_state.value()).into();
        self.wallet.set_state(Value(value), account_state.counter());
        Ok(())
    }

    pub fn get_account_state(&self) -> Result<AccountState, ControllerError> {
        self.backend.account_state(self.id()).map_err(Into::into)
    }

    pub fn vote(
        &mut self,
        proposal: &VitProposal,
        choice: Choice,
    ) -> Result<FragmentId, ControllerError> {
        let transaction =
            self.wallet
                .vote(self.settings.clone(), &proposal.clone().into(), choice)?;
        Ok(self.backend.send_fragment(transaction.to_vec())?)
    }

    pub fn update_proposals(&mut self) -> Result<(), ControllerError> {
        let proposals = self
            .get_proposals()?
            .iter()
            .cloned()
            .map(|x| x.into())
            .collect::<Vec<Proposal>>();
        self.wallet.set_proposals(proposals);
        Ok(())
    }

    pub fn get_proposals(&mut self) -> Result<Vec<VitProposal>, ControllerError> {
        Ok(self
            .backend
            .proposals()?
            .iter()
            .cloned()
            .map(Into::into)
            .collect())
    }

    pub fn fragment_logs(&self) -> Result<HashMap<FragmentId, FragmentLog>, ControllerError> {
        Ok(self.backend.fragment_logs()?)
    }

    pub fn active_votes(&self) -> Result<Vec<SimpleVoteStatus>, ControllerError> {
        Ok(self.backend.vote_statuses(self.wallet.identifier())?)
    }

    pub fn is_converted(&mut self) -> Result<bool, ControllerError> {
        self.backend
            .account_exists(self.wallet.id())
            .map_err(Into::into)
    }
}

#[derive(Debug, Error)]
pub enum ControllerError {
    #[error("wallet error")]
    WalletError(#[from] crate::wallet::WalletError),
    #[error("wallet error")]
    BackendError(#[from] crate::backend::WalletBackendError),
    #[error("transactions with ids [{fragments:?}] were pending for too long")]
    TransactionsWerePendingForTooLong { fragments: Vec<FragmentId> },
}
