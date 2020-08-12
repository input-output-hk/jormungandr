#![allow(dead_code)]

use chain_crypto::{bech32::Bech32, Ed25519, PublicKey};
use chain_impl_mockchain::fragment::FragmentId;
use jormungandr_lib::interfaces::{AccountState, FragmentLog, NodeStatsDto, VotePlanStatus};
use jormungandr_testing_utils::testing::node::JormungandrRest;
pub use jormungandr_testing_utils::testing::node::RestError;
use regex::Regex;
use std::collections::HashMap;
use std::str::FromStr;
use wallet::AccountId;
pub struct WalletNodeRestClient {
    rest_client: JormungandrRest,
}

impl WalletNodeRestClient {
    pub fn new(address: String) -> Self {
        let re = Regex::new(r"/v0/?").unwrap();
        let address = re.replace_all(&address, "");
        Self {
            rest_client: JormungandrRest::new(address.to_string()),
        }
    }

    pub fn send_fragment(&self, body: Vec<u8>) -> Result<(), RestError> {
        self.rest_client.send_raw_fragment(body)?;
        Ok(())
    }

    pub fn fragment_logs(&self) -> Result<HashMap<FragmentId, FragmentLog>, RestError> {
        Ok(self
            .rest_client
            .fragment_logs()?
            .iter()
            .map(|(id, entry)| {
                let str = id.to_string();
                (FragmentId::from_str(&str).unwrap(), entry.clone())
            })
            .collect())
    }

    pub fn disable_logs(&mut self) {
        self.rest_client.disable_logger();
    }

    pub fn stats(&self) -> Result<NodeStatsDto, RestError> {
        self.rest_client.stats()
    }

    pub fn account_state(&self, account_id: AccountId) -> Result<AccountState, RestError> {
        let public_key: PublicKey<Ed25519> = account_id.into();
        self.rest_client
            .account_state_by_pk(&public_key.to_bech32_str())
    }

    pub fn account_exists(&self, account_id: AccountId) -> Result<bool, RestError> {
        let public_key: PublicKey<Ed25519> = account_id.into();
        let response_text = self
            .rest_client
            .account_state_by_pk_raw(&public_key.to_bech32_str())?;
        Ok(!response_text.is_empty())
    }

    pub fn vote_plan_statuses(&self) -> Result<Vec<VotePlanStatus>, RestError> {
        self.rest_client.vote_plan_statuses()
    }
}
