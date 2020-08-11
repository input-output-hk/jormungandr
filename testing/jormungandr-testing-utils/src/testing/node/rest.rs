use crate::testing::node::legacy;
use crate::{testing::MemPoolCheck, wallet::Wallet};
use assert_fs::fixture::ChildPath;
use chain_impl_mockchain::fragment::{Fragment, FragmentId};
use jormungandr_lib::{
    crypto::hash::Hash,
    interfaces::{
        AccountState, EnclaveLeaderId, EpochRewardsInfo, FragmentLog, NodeStatsDto, PeerRecord,
        PeerStats, StakeDistributionDto, VotePlanStatus,
    },
};
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
            inner: legacy::BackwardCompatibleRest::new(uri, None, true),
        }
    }

    pub fn disable_logger(&mut self) {
        self.inner.disable_logger();
    }

    pub fn new_with_cert(uri: String, cert_file: &ChildPath) -> Self {
        //replace http with https
        //replace localhost ip to localhost
        let url = uri
            .replace("http://", "https://")
            .replace("127.0.0.1", "localhost");
        Self {
            inner: legacy::BackwardCompatibleRest::new(
                url,
                Some(Self::extract_certificate(cert_file.path())),
                true,
            ),
        }
    }

    fn extract_certificate<P: AsRef<Path>>(cert_file: P) -> reqwest::Certificate {
        let mut buf = Vec::new();
        let path = cert_file.as_ref().as_os_str().to_str().unwrap();
        File::open(path).unwrap().read_to_end(&mut buf).unwrap();
        reqwest::Certificate::from_der(&buf).unwrap()
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

    pub fn leaders(&self) -> Result<Vec<EnclaveLeaderId>, RestError> {
        let leaders = self.inner.leaders()?;
        let leaders: Vec<EnclaveLeaderId> = if leaders.is_empty() {
            Vec::new()
        } else {
            serde_json::from_str(&leaders).map_err(RestError::CannotDeserialize)?
        };
        Ok(leaders)
    }

    pub fn send_fragment(&self, fragment: Fragment) -> Result<MemPoolCheck, RestError> {
        self.inner.send_fragment(fragment).map_err(Into::into)
    }

    pub fn send_raw_fragment(&self, bytes: Vec<u8>) -> Result<(), RestError> {
        self.inner.send_raw_fragment(bytes)?;
        Ok(())
    }

    pub fn vote_plan_statuses(&self) -> Result<Vec<VotePlanStatus>, RestError> {
        serde_json::from_str(&self.inner.vote_plan_statuses()?)
            .map_err(RestError::CannotDeserialize)
    }
}
