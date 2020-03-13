use crate::common::configuration::jormungandr_config::JormungandrConfig;
use jormungandr_lib::interfaces::EpochRewardsInfo;
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

    fn get(&self, path: &str) -> Result<reqwest::Response, reqwest::Error> {
        reqwest::get(&format!("{}/v0/{}", self.config.get_node_address(), path))
    }
}
