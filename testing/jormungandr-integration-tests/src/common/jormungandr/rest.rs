use crate::common::{configuration::jormungandr_config::JormungandrConfig, legacy};
use chain_impl_mockchain::{fragment::FragmentId, header::HeaderId};
use jormungandr_lib::interfaces::{
    EnclaveLeaderId, EpochRewardsInfo, FragmentLog, Info, NodeStatsDto, PeerRecord, PeerStats,
    StakeDistributionDto,
};
use std::collections::HashMap;
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

/// Specialized rest api
#[derive(Debug, Clone)]
pub struct JormungandrRest {
    inner: legacy::BackwardCompatibleRest,
}

impl JormungandrRest {
    pub fn new(config: JormungandrConfig) -> Self {
        Self::from_address(config.node_config().rest.listen.to_string())
    }

    pub fn from_address(address: String) -> Self {
        let endpoint = format!("http://{}", address);
        Self {
            inner: legacy::BackwardCompatibleRest::new(endpoint),
        }
    }

    fn print_response_text(&self, text: &str) {
        println!("Response: {}", text);
    }

    pub fn epoch_reward_history(&self, epoch: u32) -> Result<EpochRewardsInfo, RestError> {
        let content = self.inner.epoch_reward_history(epoch)?;
        serde_json::from_str(&content).map_err(|err| RestError::CannotDeserialize(err))
    }

    pub fn reward_history(&self, length: u32) -> Result<Vec<EpochRewardsInfo>, RestError> {
        serde_json::from_str(&self.inner.reward_history(length)?)
            .map_err(|err| RestError::CannotDeserialize(err))
    }

    pub fn stake_distribution(&self) -> Result<StakeDistributionDto, RestError> {
        serde_json::from_str(&self.inner.stake_distribution()?)
            .map_err(|err| RestError::CannotDeserialize(err))
    }

    pub fn stake_pools(&self) -> Result<Vec<String>, RestError> {
        serde_json::from_str(&self.inner.stake_pools()?)
            .map_err(|err| RestError::CannotDeserialize(err))
    }

    pub fn stake_distribution_at(&self, epoch: u32) -> Result<StakeDistributionDto, RestError> {
        serde_json::from_str(&self.inner.stake_distribution_at(epoch)?)
            .map_err(|err| RestError::CannotDeserialize(err))
    }

    pub fn stats(&self) -> Result<NodeStatsDto, RestError> {
        let stats = &self.inner.stats()?;
        serde_json::from_str(stats).map_err(|err| RestError::CannotDeserialize(err))
    }

    pub fn network_stats(&self) -> Result<Vec<PeerStats>, RestError> {
        serde_json::from_str(&self.inner.network_stats()?)
            .map_err(|err| RestError::CannotDeserialize(err))
    }

    pub fn p2p_quarantined(&self) -> Result<Vec<PeerRecord>, RestError> {
        serde_json::from_str(&self.inner.p2p_quarantined()?)
            .map_err(|err| RestError::CannotDeserialize(err))
    }

    pub fn p2p_non_public(&self) -> Result<Vec<PeerRecord>, RestError> {
        serde_json::from_str(&self.inner.p2p_non_public()?)
            .map_err(|err| RestError::CannotDeserialize(err))
    }

    pub fn p2p_available(&self) -> Result<Vec<PeerRecord>, RestError> {
        serde_json::from_str(&self.inner.p2p_available()?)
            .map_err(|err| RestError::CannotDeserialize(err))
    }

    pub fn p2p_view(&self) -> Result<Vec<Info>, RestError> {
        serde_json::from_str(&self.inner.p2p_view()?)
            .map_err(|err| RestError::CannotDeserialize(err))
    }

    pub fn tip(&self) -> Result<HeaderId, RestError> {
        let tip = self.inner.tip()?;
        tip.parse().map_err(|err| RestError::HashParseError(err))
    }

    pub fn fragment_logs(&self) -> Result<HashMap<FragmentId, FragmentLog>, RestError> {
        let logs = self.inner.fragment_logs()?;
        let logs: Vec<FragmentLog> = if logs.is_empty() {
            Vec::new()
        } else {
            serde_json::from_str(&logs).map_err(|err| RestError::CannotDeserialize(err))?
        };

        let logs = logs
            .into_iter()
            .map(|log| (log.fragment_id().clone().into_hash(), log))
            .collect();

        Ok(logs)
    }

    pub fn leaders(&self) -> Result<Vec<EnclaveLeaderId>, RestError> {
        let leaders = self.inner.leaders()?;
        let leaders: Vec<EnclaveLeaderId> = if leaders.is_empty() {
            Vec::new()
        } else {
            serde_json::from_str(&leaders).map_err(|err| RestError::CannotDeserialize(err))?
        };
        Ok(leaders)
    }
}
