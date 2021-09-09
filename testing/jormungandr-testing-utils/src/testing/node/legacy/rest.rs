use crate::{
    testing::{
        node::{RawRest, RestError, RestSettings},
        MemPoolCheck,
    },
    wallet::Wallet,
};
use chain_core::property::Fragment as _;
use chain_impl_mockchain::fragment::{Fragment, FragmentId};
use jormungandr_lib::interfaces::{
    Address, FragmentStatus, FragmentsProcessingSummary, VotePlanId,
};
use jormungandr_lib::{crypto::hash::Hash, interfaces::FragmentLog};
use reqwest::blocking::Response;
use std::collections::HashMap;

/// Legacy tolerant rest api
/// This layer returns raw strings without deserialization
/// in order to assure compatibility and lack of serde errors
#[derive(Debug, Clone)]
pub struct BackwardCompatibleRest {
    settings: RestSettings,
    raw: RawRest,
}

impl BackwardCompatibleRest {
    pub fn new(uri: String, settings: RestSettings) -> Self {
        Self {
            settings: settings.clone(),
            raw: RawRest::new(uri, settings),
        }
    }

    pub fn raw(&self) -> &RawRest {
        &self.raw
    }

    fn print_response_text(&self, text: &str) {
        if self.settings.enable_debug {
            println!("Response: {}", text);
        }
    }

    fn print_debug_response(&self, response: &Response) {
        if self.settings.enable_debug {
            println!("Response: {:?}", response);
        }
    }

    pub fn disable_logger(&mut self) {
        self.raw.disable_logger();
        self.settings.enable_debug = false;
    }

    pub fn enable_logger(&mut self) {
        self.raw.enable_logger();
        self.settings.enable_debug = true;
    }

    pub fn epoch_reward_history(&self, epoch: u32) -> Result<String, reqwest::Error> {
        let response_text = self.raw().epoch_reward_history(epoch)?.text()?;
        self.print_response_text(&response_text);
        Ok(response_text)
    }

    pub fn reward_history(&self, length: u32) -> Result<String, reqwest::Error> {
        let response_text = self.raw().reward_history(length)?.text()?;
        self.print_response_text(&response_text);
        Ok(response_text)
    }

    pub fn stake_distribution(&self) -> Result<String, reqwest::Error> {
        let response_text = self.raw().stake_distribution()?.text()?;
        self.print_response_text(&response_text);
        Ok(response_text)
    }

    pub fn account_state(&self, wallet: &Wallet) -> Result<String, reqwest::Error> {
        self.account_state_by_pk(&wallet.identifier().to_bech32_str())
    }

    pub fn account_state_by_pk(&self, bech32_str: &str) -> Result<String, reqwest::Error> {
        let response_text = self.raw().account_state_by_pk(bech32_str)?.text()?;
        self.print_response_text(&response_text);
        Ok(response_text)
    }

    pub fn stake_pools(&self) -> Result<String, reqwest::Error> {
        let response_text = self.raw().stake_pools()?.text()?;
        self.print_response_text(&response_text);
        Ok(response_text)
    }

    pub fn stake_distribution_at(&self, epoch: u32) -> Result<String, reqwest::Error> {
        let response_text = self.raw().stake_distribution_at(epoch)?.text()?;
        self.print_response_text(&response_text);
        Ok(response_text)
    }

    pub fn stats(&self) -> Result<String, reqwest::Error> {
        self.raw().stats()?.text()
    }

    pub fn network_stats(&self) -> Result<String, reqwest::Error> {
        self.raw().network_stats()?.text()
    }

    pub fn p2p_quarantined(&self) -> Result<String, reqwest::Error> {
        self.raw().p2p_quarantined()?.text()
    }

    pub fn p2p_non_public(&self) -> Result<String, reqwest::Error> {
        self.raw().p2p_non_public()?.text()
    }

    pub fn p2p_available(&self) -> Result<String, reqwest::Error> {
        self.raw().p2p_available()?.text()
    }

    pub fn p2p_view(&self) -> Result<String, reqwest::Error> {
        self.raw().p2p_view()?.text()
    }

    pub fn leaders_log(&self) -> Result<String, reqwest::Error> {
        self.raw().leaders_log()?.text()
    }

    pub fn tip(&self) -> Result<Hash, RestError> {
        let tip = self.raw().tip()?.text()?;
        tip.parse().map_err(RestError::HashParseError)
    }

    pub fn settings(&self) -> Result<String, reqwest::Error> {
        self.raw().settings()?.text()
    }

    pub fn fragments_statuses(
        &self,
        ids: Vec<String>,
    ) -> Result<HashMap<String, FragmentStatus>, RestError> {
        let logs = self.raw().fragments_statuses(ids)?.text()?;
        serde_json::from_str(&logs).map_err(RestError::CannotDeserialize)
    }

    pub fn fragments_logs(&self) -> Result<HashMap<FragmentId, FragmentLog>, RestError> {
        let logs = self.raw().fragments_logs()?.text()?;
        let logs: Vec<FragmentLog> = if logs.is_empty() {
            Vec::new()
        } else {
            serde_json::from_str(&logs).map_err(RestError::CannotDeserialize)?
        };

        let logs = logs
            .into_iter()
            .map(|log| ((*log.fragment_id()).into_hash(), log))
            .collect();

        Ok(logs)
    }

    pub fn fragment_logs(&self) -> Result<HashMap<FragmentId, FragmentLog>, RestError> {
        let logs = self.raw().fragment_logs()?.text()?;
        let logs: Vec<FragmentLog> = if logs.is_empty() {
            Vec::new()
        } else {
            serde_json::from_str(&logs).map_err(RestError::CannotDeserialize)?
        };

        let logs = logs
            .into_iter()
            .map(|log| ((*log.fragment_id()).into_hash(), log))
            .collect();

        Ok(logs)
    }

    pub fn send_fragment(&self, fragment: Fragment) -> Result<MemPoolCheck, reqwest::Error> {
        let fragment_id = fragment.id();
        let response = self.raw().send_fragment(fragment)?;
        self.print_response_text(&response.text()?);
        Ok(MemPoolCheck::new(fragment_id))
    }

    pub fn send_raw_fragment(
        &self,
        body: Vec<u8>,
    ) -> Result<reqwest::blocking::Response, reqwest::Error> {
        let response = self.raw.send_raw_fragment(body)?;
        self.print_debug_response(&response);
        Ok(response)
    }

    pub fn send_raw_fragments(&self, bodies: Vec<Vec<u8>>) -> Result<(), reqwest::Error> {
        self.raw.send_raw_fragments(bodies)
    }
    pub fn send_fragment_batch(
        &self,
        fragments: Vec<Fragment>,
        fail_fast: bool,
    ) -> Result<FragmentsProcessingSummary, RestError> {
        let checks: Vec<MemPoolCheck> = fragments
            .iter()
            .map(|x| MemPoolCheck::new(x.id()))
            .collect();
        let response = self.raw.send_fragment_batch(fragments, fail_fast)?;
        self.print_debug_response(&response);
        if response.status() == reqwest::StatusCode::OK {
            Ok(serde_json::from_str(&response.text()?)?)
        } else {
            Err(RestError::NonSuccessErrorCode {
                status: response.status(),
                response: response.text().unwrap(),
                checks,
            })
        }
    }

    pub fn vote_plan_statuses(&self) -> Result<String, reqwest::Error> {
        self.raw().vote_plan_statuses()?.text()
    }

    pub fn vote_plan_account_info(
        &self,
        vote_plan_id: VotePlanId,
        address: Address,
    ) -> Result<String, reqwest::Error> {
        self.raw()
            .vote_plan_account_info(vote_plan_id, address)?
            .text()
    }
}
