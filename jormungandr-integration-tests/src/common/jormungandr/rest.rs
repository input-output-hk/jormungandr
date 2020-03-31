use crate::common::configuration::jormungandr_config::JormungandrConfig;
use jormungandr_lib::interfaces::{EpochRewardsInfo, Info, NodeStatsDto, PeerRecord, PeerStats};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RestError {
    #[error("could not deserialize response")]
    CannotDeserialize(#[from] serde_json::Error),
    #[error("could not send reqeuest")]
    SendRequestError(#[from] reqwest::Error),
}

#[derive(Debug)]
pub struct JormungandrRest {
    config: JormungandrConfig,
}

impl JormungandrRest {
    pub fn new(config: JormungandrConfig) -> Self {
        Self { config: config }
    }

    pub fn epoch_reward_history(&self, epoch: u32) -> Result<EpochRewardsInfo, RestError> {
        let request = format!("rewards/epoch/{}", epoch);
        let response_text = self.get(&request)?.text()?;
        serde_json::from_str(&response_text).map_err(|err| RestError::CannotDeserialize(err))
    }

    pub fn reward_history(&self, length: u32) -> Result<Vec<EpochRewardsInfo>, RestError> {
        let request = format!("rewards/history/{}", length);
        let response_text = self.get(&request)?.text()?;
        serde_json::from_str(&response_text).map_err(|err| RestError::CannotDeserialize(err))
    }

    fn get(&self, path: &str) -> Result<reqwest::blocking::Response, reqwest::Error> {
        reqwest::blocking::get(&format!("{}/v0/{}", self.config.get_node_address(), path))
    }

    pub fn stats(&self) -> Result<NodeStatsDto, RestError> {
        let response_text = self.get("node/stats")?.text()?;
        println!("network/stats: {}", response_text);
        serde_json::from_str(&response_text).map_err(|err| RestError::CannotDeserialize(err))
    }

    pub fn network_stats(&self) -> Result<Vec<PeerStats>, RestError> {
        let response_text = self.get("network/stats")?.text()?;
        println!("network/stats: {}", response_text);
        serde_json::from_str(&response_text).map_err(|err| RestError::CannotDeserialize(err))
    }

    pub fn p2p_quarantined(&self) -> Result<Vec<PeerRecord>, RestError> {
        let response_text = self.get("network/p2p/quarantined")?.text()?;
        println!("network/p2p_quarantined: {}", response_text);
        serde_json::from_str(&response_text).map_err(|err| RestError::CannotDeserialize(err))
    }

    pub fn p2p_non_public(&self) -> Result<Vec<PeerRecord>, RestError> {
        let response_text = self.get("network/p2p/non_public")?.text()?;
        println!("nnetwork/non_public: {}", response_text);
        serde_json::from_str(&response_text).map_err(|err| RestError::CannotDeserialize(err))
    }

    pub fn p2p_available(&self) -> Result<Vec<PeerRecord>, RestError> {
        let response_text = self.get("network/p2p/available")?.text()?;
        println!("network/available: {}", response_text);
        serde_json::from_str(&response_text).map_err(|err| RestError::CannotDeserialize(err))
    }

    pub fn p2p_view(&self) -> Result<Vec<Info>, RestError> {
        let response_text = self.get("network/p2p/view")?.text()?;
        println!("network/p2p/view: {}", response_text);
        serde_json::from_str(&response_text).map_err(|err| RestError::CannotDeserialize(err))
    }
}
