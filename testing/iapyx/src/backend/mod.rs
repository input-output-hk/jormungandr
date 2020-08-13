mod node;
mod proxy;
mod vit_station;

use crate::Proposal;
use crate::SimpleVoteStatus;
use chain_core::mempack::Readable;
use chain_core::{mempack::ReadBuf, property::Fragment as _};
use chain_impl_mockchain::{
    block::Block,
    fragment::{Fragment, FragmentId},
};
use chain_ser::deser::Deserialize;
use jormungandr_lib::interfaces::AccountIdentifier;
use jormungandr_lib::interfaces::{AccountState, FragmentLog, VotePlanStatus};
use jormungandr_testing_utils::testing::node::Explorer;
use jormungandr_testing_utils::testing::node::RestSettings;
use node::{RestError as NodeRestError, WalletNodeRestClient};
use proxy::{Error as ProxyError, ProxyClient};
use std::collections::HashMap;
use std::str::FromStr;
use thiserror::Error;
use vit_station::{RestError as VitRestError, VitStationRestClient};
use wallet::{AccountId, Settings};

pub struct WalletBackend {
    node_client: WalletNodeRestClient,
    vit_client: VitStationRestClient,
    proxy_client: ProxyClient,
    explorer_client: Explorer,
}

impl WalletBackend {
    pub fn new(address: String, node_rest_settings: RestSettings) -> Self {
        let mut backend = Self {
            node_client: WalletNodeRestClient::new(
                format!("http://{}/api", address),
                node_rest_settings.clone(),
            ),
            vit_client: VitStationRestClient::new(address.clone()),
            proxy_client: ProxyClient::new(format!("http://{}/api/v0", address)),
            explorer_client: Explorer::new(address),
        };

        if node_rest_settings.enable_debug {
            backend.enable_logs()
        }
        backend
    }

    pub fn send_fragment(&self, transaction: Vec<u8>) -> Result<FragmentId, WalletBackendError> {
        self.node_client.send_fragment(transaction.clone())?;
        let fragment = Fragment::deserialize(transaction.as_slice())?;
        Ok(fragment.id())
    }

    pub fn fragment_logs(&self) -> Result<HashMap<FragmentId, FragmentLog>, WalletBackendError> {
        self.node_client.fragment_logs().map_err(Into::into)
    }

    pub fn account_state(&self, account_id: AccountId) -> Result<AccountState, WalletBackendError> {
        self.node_client
            .account_state(account_id)
            .map_err(Into::into)
    }

    pub fn proposals(&self) -> Result<Vec<Proposal>, WalletBackendError> {
        Ok(self
            .vit_client
            .proposals()?
            .iter()
            .cloned()
            .map(Into::into)
            .collect())
    }

    pub fn block0(&self) -> Result<Vec<u8>, WalletBackendError> {
        Ok(self.proxy_client.block0().map(Into::into)?)
    }

    pub fn vote_plan_statuses(&self) -> Result<Vec<VotePlanStatus>, WalletBackendError> {
        self.node_client.vote_plan_statuses().map_err(Into::into)
    }

    pub fn disable_logs(&mut self) {
        self.node_client.disable_logs();
        self.vit_client.disable_logs();
        self.proxy_client.disable_debug();
    }

    pub fn enable_logs(&mut self) {
        self.node_client.enable_logs();
        self.vit_client.enable_logs();
        self.proxy_client.enable_debug();
    }

    pub fn are_fragments_in_blockchain(
        &self,
        fragment_ids: Vec<FragmentId>,
    ) -> Result<bool, WalletBackendError> {
        Ok(fragment_ids.iter().all(|x| {
            let hash = jormungandr_lib::crypto::hash::Hash::from_str(&x.to_string()).unwrap();
            self.explorer_client.get_transaction(hash).is_ok()
        }))
    }

    pub fn vote_statuses(
        &self,
        identifier: AccountIdentifier,
    ) -> Result<Vec<SimpleVoteStatus>, WalletBackendError> {
        let vote_plan_statuses = self.vote_plan_statuses().unwrap();
        let proposals = self.proposals().unwrap();

        let mut active_votes = Vec::new();
        for vote_plan_status in vote_plan_statuses {
            for proposal in vote_plan_status.proposals {
                for (account, payload) in proposal.votes.iter() {
                    if *account == identifier {
                        let vit_proposal = proposals
                            .iter()
                            .find(|x| {
                                x.chain_proposal_id_as_str()
                                    == proposal.proposal_id.clone().to_string()
                            })
                            .unwrap();
                        active_votes.push(SimpleVoteStatus {
                            chain_proposal_id: vit_proposal.chain_proposal_id_as_str(),
                            proposal_title: vit_proposal.proposal_title.clone(),
                            choice: vit_proposal.get_option_text(payload.choice()),
                        });
                    }
                }
            }
        }
        Ok(active_votes)
    }

    pub fn settings(&self) -> Result<Settings, WalletBackendError> {
        let block0 = self.block0()?;
        let mut block0_bytes = ReadBuf::from(&block0);
        let block0 =
            Block::read(&mut block0_bytes).map_err(|e| WalletBackendError::Block0ReadError(e))?;
        Ok(Settings::new(&block0).map_err(|e| WalletBackendError::SettingsReadError(e))?)
    }

    pub fn account_exists(&self, id: AccountId) -> Result<bool, WalletBackendError> {
        self.node_client.account_exists(id).map_err(Into::into)
    }
}

#[derive(Debug, Error)]
pub enum WalletBackendError {
    #[error("vit station error")]
    VitStationConnectionError(#[from] VitRestError),
    #[error("node rest error")]
    NodeConnectionError(#[from] NodeRestError),
    #[error("node rest error")]
    ProxyConnectionError(#[from] ProxyError),
    #[error("io error")]
    IOError(#[from] std::io::Error),
    #[error("block0 retrieve error")]
    Block0ReadError(#[from] chain_core::mempack::ReadError),
    #[error("block0 retrieve error")]
    SettingsReadError(#[from] chain_impl_mockchain::ledger::Error),
}
