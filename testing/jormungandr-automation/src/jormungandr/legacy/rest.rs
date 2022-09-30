use crate::jormungandr::{MemPoolCheck, RawRest, RestError, RestSettings};
use chain_core::property::Fragment as _;
#[cfg(feature = "evm")]
use chain_evm::Address as EvmAddress;
#[cfg(feature = "evm")]
use chain_impl_mockchain::account::Identifier as JorAddress;
use chain_impl_mockchain::{
    fragment::{Fragment, FragmentId},
    header::HeaderId,
};
use jormungandr_lib::{
    crypto::{account::Identifier, hash::Hash},
    interfaces::{Address, FragmentLog, FragmentStatus, FragmentsProcessingSummary, VotePlanId},
};
use reqwest::blocking::Response;
use std::collections::HashMap;

/// Legacy tolerant rest api
/// This layer returns raw strings without deserialization
/// in order to assure compatibility and lack of serde errors
#[derive(Debug, Clone)]
pub struct BackwardCompatibleRest {
    raw: RawRest,
}

impl BackwardCompatibleRest {
    pub fn new(uri: String, settings: RestSettings) -> Self {
        Self {
            raw: RawRest::new(uri, settings),
        }
    }

    pub fn raw(&self) -> &RawRest {
        &self.raw
    }

    pub fn rest_settings(&self) -> &RestSettings {
        self.raw.rest_settings()
    }

    fn print_response_text(&self, text: &str) {
        if self.rest_settings().enable_debug {
            println!("Response: {}", text);
        }
    }

    fn print_debug_response(&self, response: &Response) {
        if self.rest_settings().enable_debug {
            println!("Response: {:?}", response);
        }
    }

    pub fn disable_logger(&mut self) {
        self.raw.disable_logger();
        self.raw.rest_settings_mut().enable_debug = false;
    }

    pub fn enable_logger(&mut self) {
        self.raw.enable_logger();
        self.raw.rest_settings_mut().enable_debug = true;
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

    pub fn remaining_rewards(&self) -> Result<String, reqwest::Error> {
        let response_text = self.raw().remaining_rewards()?.text()?;
        self.print_response_text(&response_text);
        Ok(response_text)
    }

    pub fn stake_distribution(&self) -> Result<String, reqwest::Error> {
        let response_text = self.raw().stake_distribution()?.text()?;
        self.print_response_text(&response_text);
        Ok(response_text)
    }

    pub fn account_votes_all(&self) -> Result<String, reqwest::Error> {
        let response_text = self.raw().account_votes_all()?.text()?;
        self.print_response_text(&response_text);
        Ok(response_text)
    }

    pub fn account_state(&self, id: &Identifier) -> Result<String, reqwest::Error> {
        self.account_state_by_pk(&id.to_bech32_str())
    }

    pub fn account_votes(&self, wallet_address: Address) -> Result<String, reqwest::Error> {
        let response_text = self.raw().account_votes(wallet_address)?.text()?;
        self.print_response_text(&response_text);
        Ok(response_text)
    }

    pub fn account_votes_with_plan_id(
        &self,
        vote_plan_id: VotePlanId,
        wallet_address: Address,
    ) -> Result<String, reqwest::Error> {
        let response_text = self
            .raw()
            .account_votes_with_plan_id(vote_plan_id, wallet_address)?
            .text()?;
        self.print_response_text(&response_text);
        Ok(response_text)
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

    #[cfg(feature = "evm")]
    pub fn evm_address(&self, jor_address: &JorAddress) -> Result<String, reqwest::Error> {
        let response_text = self.raw().evm_address(jor_address)?.text()?;
        self.print_response_text(&response_text);
        Ok(response_text)
    }

    #[cfg(feature = "evm")]
    pub fn jor_address(&self, evm_address: &EvmAddress) -> Result<String, reqwest::Error> {
        let response_text = self.raw().jor_address(evm_address)?.text()?;
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

    pub fn block_as_bytes(&self, header_hash: &HeaderId) -> Result<Vec<u8>, RestError> {
        let mut bytes = Vec::new();
        let mut resp = self.raw().block(header_hash)?;
        resp.copy_to(&mut bytes)?;
        Ok(bytes)
    }

    pub fn shutdown(&self) -> Result<String, reqwest::Error> {
        self.raw().shutdown()?.text()
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

    pub fn set_origin<S: Into<String>>(&mut self, origin: S) {
        self.raw.rest_settings_mut().cors = Some(origin.into());
    }
}
