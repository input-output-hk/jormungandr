use crate::common::{configuration::jormungandr_config::JormungandrConfig, legacy};
use jormungandr_lib::interfaces::{
    EpochRewardsInfo, Info, NodeStatsDto, PeerRecord, PeerStats, StakeDistributionDto,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RestError {
    #[error("could not deserialize response")]
    CannotDeserialize(#[from] serde_json::Error),
    #[error("could not send reqeuest")]
    RequestError(#[from] reqwest::Error),
}

/// Specialized rest api
#[derive(Debug)]
pub struct JormungandrRest {
    inner: legacy::BackwardCompatibleRest,
}

impl JormungandrRest {
    pub fn new(config: JormungandrConfig) -> Self {
        Self {
            inner: legacy::BackwardCompatibleRest::new(config.get_node_address()),
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
        serde_json::from_str(&self.inner.stats()?).map_err(|err| RestError::CannotDeserialize(err))
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
}
