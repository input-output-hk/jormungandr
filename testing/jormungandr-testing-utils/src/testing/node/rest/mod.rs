mod load;
mod raw;
mod settings;

use crate::{testing::node::legacy, testing::MemPoolCheck, wallet::Wallet};
use chain_impl_mockchain::block::Block;
use chain_impl_mockchain::fragment::{Fragment, FragmentId};
use chain_impl_mockchain::header::HeaderId;
use jormungandr_lib::interfaces::{
    Address, FragmentStatus, FragmentsProcessingSummary, VotePlanId,
};
use jormungandr_lib::{
    crypto::hash::Hash,
    interfaces::{
        AccountState, EpochRewardsInfo, FragmentLog, LeadershipLog, NodeStatsDto, PeerRecord,
        PeerStats, SettingsDto, StakeDistributionDto, VotePlanStatus,
    },
};
pub use load::RestRequestGen;
pub use raw::RawRest;
pub use settings::RestSettings;
use std::collections::HashMap;
use std::io::Read;
use std::{fs::File, net::SocketAddr, path::Path};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RestError {
    #[error("could not deserialize response")]
    CannotDeserialize(#[from] serde_json::Error),
    #[error("could not send reqeuest")]
    RequestError(#[from] reqwest::Error),
    #[error("hash parse error")]
    HashParseError(#[from] chain_crypto::hash::Error),
    #[error("error while polling endpoint")]
    PollError(#[from] jortestkit::process::WaitError),
    #[error("non success error code {status}")]
    NonSuccessErrorCode {
        response: String,
        status: reqwest::StatusCode,
        checks: Vec<MemPoolCheck>,
    },
    #[error(transparent)]
    ReadBytes(#[from] chain_core::mempack::ReadError),
}

pub fn uri_from_socket_addr(addr: SocketAddr) -> String {
    format!("http://{}/api", addr)
}

/// Specialized rest api
#[derive(Debug, Clone)]
pub struct JormungandrRest {
    inner: legacy::BackwardCompatibleRest,
}

impl JormungandrRest {
    pub fn new(uri: String) -> Self {
        Self {
            inner: legacy::BackwardCompatibleRest::new(uri, Default::default()),
        }
    }

    pub fn new_with_custom_settings(uri: String, settings: RestSettings) -> Self {
        Self {
            inner: legacy::BackwardCompatibleRest::new(uri, settings),
        }
    }

    pub fn disable_logger(&mut self) {
        self.inner.disable_logger();
    }

    pub fn enable_logger(&mut self) {
        self.inner.enable_logger();
    }

    pub fn inner(&self) -> &legacy::BackwardCompatibleRest {
        &self.inner
    }

    pub fn raw(&self) -> &RawRest {
        self.inner.raw()
    }

    pub fn new_with_cert<P: AsRef<Path>>(uri: String, cert_file: P) -> Self {
        //replace http with https
        //replace localhost ip to localhost
        let url = uri
            .replace("http://", "https://")
            .replace("127.0.0.1", "localhost");

        let settings = RestSettings {
            certificate: Some(Self::extract_certificate(cert_file.as_ref())),
            ..Default::default()
        };
        Self {
            inner: legacy::BackwardCompatibleRest::new(url, settings),
        }
    }

    fn extract_certificate<P: AsRef<Path>>(cert_file: P) -> reqwest::Certificate {
        let mut buf = Vec::new();
        let path = cert_file.as_ref().as_os_str().to_str().unwrap();
        File::open(path).unwrap().read_to_end(&mut buf).unwrap();
        reqwest::Certificate::from_pem(&buf).unwrap()
    }

    pub fn epoch_reward_history(&self, epoch: u32) -> Result<EpochRewardsInfo, RestError> {
        let content = self.inner.epoch_reward_history(epoch)?;
        serde_json::from_str(&content).map_err(RestError::CannotDeserialize)
    }

    pub fn reward_history(&self, length: u32) -> Result<Vec<EpochRewardsInfo>, RestError> {
        serde_json::from_str(&self.inner.reward_history(length)?)
            .map_err(RestError::CannotDeserialize)
    }

    pub fn stake_distribution(&self) -> Result<StakeDistributionDto, RestError> {
        serde_json::from_str(&self.inner.stake_distribution()?)
            .map_err(RestError::CannotDeserialize)
    }

    pub fn account_votes(
        &self,
        vote_plan_id: VotePlanId,
        wallet: &Wallet,
    ) -> Result<Option<Vec<u8>>, RestError> {
        serde_json::from_str(&self.inner.account_votes(vote_plan_id, wallet.address())?)
            .map_err(RestError::CannotDeserialize)
    }

    pub fn stake_pools(&self) -> Result<Vec<String>, RestError> {
        serde_json::from_str(&self.inner.stake_pools()?).map_err(RestError::CannotDeserialize)
    }

    pub fn stake_distribution_at(&self, epoch: u32) -> Result<StakeDistributionDto, RestError> {
        serde_json::from_str(&self.inner.stake_distribution_at(epoch)?)
            .map_err(RestError::CannotDeserialize)
    }

    pub fn stats(&self) -> Result<NodeStatsDto, RestError> {
        let stats = &self.inner.stats()?;
        serde_json::from_str(stats).map_err(RestError::CannotDeserialize)
    }

    pub fn account_state(&self, wallet: &Wallet) -> Result<AccountState, RestError> {
        serde_json::from_str(&self.inner.account_state(wallet)?)
            .map_err(RestError::CannotDeserialize)
    }

    pub fn account_state_by_pk_raw(&self, bech32_str: &str) -> Result<String, RestError> {
        self.inner
            .account_state_by_pk(bech32_str)
            .map_err(Into::into)
    }

    pub fn account_state_by_pk(&self, bech32_str: &str) -> Result<AccountState, RestError> {
        serde_json::from_str(&self.inner.account_state_by_pk(bech32_str)?)
            .map_err(RestError::CannotDeserialize)
    }

    pub fn network_stats(&self) -> Result<Vec<PeerStats>, RestError> {
        serde_json::from_str(&self.inner.network_stats()?).map_err(RestError::CannotDeserialize)
    }

    pub fn p2p_quarantined(&self) -> Result<Vec<PeerRecord>, RestError> {
        serde_json::from_str(&self.inner.p2p_quarantined()?).map_err(RestError::CannotDeserialize)
    }

    pub fn p2p_non_public(&self) -> Result<Vec<PeerRecord>, RestError> {
        serde_json::from_str(&self.inner.p2p_non_public()?).map_err(RestError::CannotDeserialize)
    }

    pub fn p2p_available(&self) -> Result<Vec<PeerRecord>, RestError> {
        serde_json::from_str(&self.inner.p2p_available()?).map_err(RestError::CannotDeserialize)
    }

    pub fn p2p_view(&self) -> Result<Vec<String>, RestError> {
        serde_json::from_str(&self.inner.p2p_view()?).map_err(RestError::CannotDeserialize)
    }

    pub fn tip(&self) -> Result<Hash, RestError> {
        self.inner.tip()
    }

    pub fn fragment_logs(&self) -> Result<HashMap<FragmentId, FragmentLog>, RestError> {
        self.inner.fragment_logs()
    }

    pub fn settings(&self) -> Result<SettingsDto, RestError> {
        serde_json::from_str(&self.inner.settings()?).map_err(RestError::CannotDeserialize)
    }

    pub fn leaders_log(&self) -> Result<Vec<LeadershipLog>, RestError> {
        serde_json::from_str(&self.inner.leaders_log()?).map_err(RestError::CannotDeserialize)
    }

    pub fn send_fragment(&self, fragment: Fragment) -> Result<MemPoolCheck, RestError> {
        self.inner.send_fragment(fragment).map_err(Into::into)
    }

    pub fn send_raw_fragment(&self, bytes: Vec<u8>) -> Result<(), RestError> {
        self.inner.send_raw_fragment(bytes)?;
        Ok(())
    }

    pub fn send_raw_fragments(&self, bytes: Vec<Vec<u8>>) -> Result<(), RestError> {
        self.inner.send_raw_fragments(bytes).map_err(Into::into)
    }

    pub fn block_as_bytes(&self, header_hash: &HeaderId) -> Result<Vec<u8>, RestError> {
        self.inner.block_as_bytes(header_hash).map_err(Into::into)
    }

    pub fn shutdown(&self) -> Result<String, RestError> {
        self.inner.shutdown().map_err(Into::into)
    }

    pub fn block(&self, header_hash: &HeaderId) -> Result<Block, RestError> {
        use chain_core::mempack::{ReadBuf, Readable as _};
        let bytes = self.block_as_bytes(header_hash)?;
        Block::read(&mut ReadBuf::from(&bytes)).map_err(Into::into)
    }

    pub fn fragments_statuses(
        &self,
        ids: Vec<String>,
    ) -> Result<HashMap<String, FragmentStatus>, RestError> {
        self.inner.fragments_statuses(ids).map_err(Into::into)
    }

    pub fn send_fragment_batch(
        &self,
        fragments: Vec<Fragment>,
        fail_fast: bool,
    ) -> Result<FragmentsProcessingSummary, RestError> {
        self.inner
            .send_fragment_batch(fragments, fail_fast)
            .map_err(Into::into)
    }

    pub fn vote_plan_statuses(&self) -> Result<Vec<VotePlanStatus>, RestError> {
        serde_json::from_str(&self.inner.vote_plan_statuses()?)
            .map_err(RestError::CannotDeserialize)
    }

    pub fn set_origin<S: Into<String>>(&mut self, origin: S) {
        self.inner.set_origin(origin);
    }

    pub fn vote_plan_account_info(
        &self,
        vote_plan_id: VotePlanId,
        address: Address,
    ) -> Result<Vec<u8>, RestError> {
        serde_json::from_str(&self.inner.vote_plan_account_info(vote_plan_id, address)?)
            .map_err(RestError::CannotDeserialize)
    }
}
