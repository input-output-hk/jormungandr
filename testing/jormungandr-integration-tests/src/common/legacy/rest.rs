use crate::common::jormungandr::rest::RestError;
use chain_impl_mockchain::fragment::{Fragment, FragmentId};
use jormungandr_lib::interfaces::FragmentLog;
use jormungandr_testing_utils::testing::MemPoolCheck;
use std::collections::HashMap;

/// Legacy tolerant rest api
/// This layer returns raw strings without deserialization
/// in order to assure compatibility and lack of serde errors
#[derive(Debug, Clone)]
pub struct BackwardCompatibleRest {
    endpoint: String,
}

impl BackwardCompatibleRest {
    pub fn new(endpoint: String) -> Self {
        Self { endpoint }
    }

    fn print_response_text(&self, text: &str) {
        println!("Response: {}", text);
    }

    pub fn epoch_reward_history(&self, epoch: u32) -> Result<String, reqwest::Error> {
        let request = format!("rewards/epoch/{}", epoch);
        let response_text = self.get(&request)?.text()?;
        self.print_response_text(&response_text);
        Ok(response_text)
    }

    pub fn reward_history(&self, length: u32) -> Result<String, reqwest::Error> {
        let request = format!("rewards/history/{}", length);
        let response_text = self.get(&request)?.text()?;
        self.print_response_text(&response_text);
        Ok(response_text)
    }

    fn get(&self, path: &str) -> Result<reqwest::blocking::Response, reqwest::Error> {
        let request = self.path(path);
        reqwest::blocking::get(&request)
    }

    fn path(&self, path: &str) -> String {
        format!("{}/api/v0/{}", self.endpoint, path)
    }

    pub fn stake_distribution(&self) -> Result<String, reqwest::Error> {
        let response_text = self.get("stake")?.text()?;
        self.print_response_text(&response_text);
        Ok(response_text)
    }

    pub fn stake_pools(&self) -> Result<String, reqwest::Error> {
        let response_text = self.get("stake_pools")?.text()?;
        self.print_response_text(&response_text);
        Ok(response_text)
    }

    pub fn stake_distribution_at(&self, epoch: u32) -> Result<String, reqwest::Error> {
        let request = format!("stake/{}", epoch);
        let response_text = self.get(&request)?.text()?;
        self.print_response_text(&response_text);
        Ok(response_text)
    }

    pub fn stats(&self) -> Result<String, reqwest::Error> {
        self.get("node/stats")?.text()
    }

    pub fn network_stats(&self) -> Result<String, reqwest::Error> {
        self.get("network/stats")?.text()
    }

    pub fn p2p_quarantined(&self) -> Result<String, reqwest::Error> {
        self.get("network/p2p/quarantined")?.text()
    }

    pub fn p2p_non_public(&self) -> Result<String, reqwest::Error> {
        self.get("network/p2p/non_public")?.text()
    }

    pub fn p2p_available(&self) -> Result<String, reqwest::Error> {
        self.get("network/p2p/available")?.text()
    }

    pub fn p2p_view(&self) -> Result<String, reqwest::Error> {
        self.get("network/p2p/view")?.text()
    }

    pub fn tip(&self) -> Result<String, reqwest::Error> {
        self.get("tip")?.text()
    }

    pub fn fragment_logs(&self) -> Result<HashMap<FragmentId, FragmentLog>, RestError> {
        let logs = self.get("fragment/logs")?.text()?;
        let logs: Vec<FragmentLog> = if logs.is_empty() {
            Vec::new()
        } else {
            serde_json::from_str(&logs).map_err(RestError::CannotDeserialize)?
        };

        let logs = logs
            .into_iter()
            .map(|log| (log.fragment_id().clone().into_hash(), log))
            .collect();

        Ok(logs)
    }

    pub fn leaders(&self) -> Result<String, reqwest::Error> {
        self.get("leaders")?.text()
    }

    fn post(
        &self,
        path: &str,
        body: Vec<u8>,
    ) -> Result<reqwest::blocking::Response, reqwest::Error> {
        let client = reqwest::blocking::Client::new();
        client.post(&self.path(path)).body(body).send()
    }

    pub fn send_fragment(&self, fragment: Fragment) -> Result<MemPoolCheck, reqwest::Error> {
        use chain_core::property::Fragment as _;
        use chain_core::property::Serialize as _;

        let raw = fragment.serialize_as_vec().unwrap();
        let fragment_id = fragment.id();

        self.post("message", raw)?;
        Ok(MemPoolCheck::new(fragment_id))
    }
}
