use super::RestSettings;
use crate::{testing::node::RestError, wallet::Wallet};
use bech32::FromBase32;
use chain_core::property::Serialize;
use chain_crypto::PublicKey;
use chain_impl_mockchain::account;
use chain_impl_mockchain::fragment::Fragment;
use jormungandr_lib::interfaces::FragmentsBatch;
use jortestkit::process::Wait;
use reqwest::{
    blocking::{Client, Response},
    header::{HeaderMap, HeaderValue, CONTENT_TYPE},
};
use std::fmt;

enum ApiVersion {
    V0,
    V1,
}

impl fmt::Display for ApiVersion {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ApiVersion::V0 => write!(f, "v0"),
            ApiVersion::V1 => write!(f, "v1"),
        }
    }
}

/// struct intends to return raw reqwest response
/// can be used to verify requests error codes or
/// to poll until data is available
#[derive(Debug, Clone)]
pub struct RawRest {
    uri: String,
    client: Client,
    logging_enabled: bool,
}

impl RawRest {
    pub fn new(uri: String, settings: RestSettings) -> Self {
        let client = match &settings.certificate {
            None => Client::new(),
            Some(cert) => Client::builder()
                .use_rustls_tls()
                .add_root_certificate(cert.clone())
                .build()
                .unwrap(),
        };
        let uri = if settings.use_https {
            let url = url::Url::parse(&uri).unwrap();
            format!(
                "https://{}:443/{}",
                url.domain().unwrap(),
                url.path_segments().unwrap().next().unwrap()
            )
        } else {
            uri
        };

        Self {
            uri,
            client,
            logging_enabled: settings.enable_debug,
        }
    }

    pub fn enable_logger(&mut self) {
        self.logging_enabled = true;
    }

    pub fn disable_logger(&mut self) {
        self.logging_enabled = false;
    }

    pub fn epoch_reward_history(&self, epoch: u32) -> Result<Response, reqwest::Error> {
        let request = format!("rewards/epoch/{}", epoch);
        self.get(&request)
    }

    pub fn reward_history(&self, length: u32) -> Result<Response, reqwest::Error> {
        let request = format!("rewards/history/{}", length);
        self.get(&request)
    }

    fn print_request_path(&self, text: &str) {
        if self.logging_enabled {
            println!("Request: {}", text);
        }
    }

    fn get(&self, path: &str) -> Result<reqwest::blocking::Response, reqwest::Error> {
        let request = self.path(path, ApiVersion::V0);
        self.print_request_path(&request);
        self.client.get(request).send()
    }

    fn path(&self, path: &str, api_version: ApiVersion) -> String {
        format!("{}/{}/{}", self.uri, api_version, path)
    }

    pub fn stake_distribution(&self) -> Result<Response, reqwest::Error> {
        self.get("stake")
    }

    pub fn account_state(&self, wallet: &Wallet) -> Result<Response, reqwest::Error> {
        self.account_state_by_pk(&wallet.identifier().to_bech32_str())
    }

    pub fn account_state_by_pk(&self, bech32_str: &str) -> Result<Response, reqwest::Error> {
        let key = hex::encode(Self::try_from_str(bech32_str).as_ref().as_ref());
        self.get(&format!("account/{}", key))
    }

    fn try_from_str(src: &str) -> account::Identifier {
        let (_, data) = bech32::decode(src).unwrap();
        let dat = Vec::from_base32(&data).unwrap();
        let pk = PublicKey::from_binary(&dat).unwrap();
        account::Identifier::from(pk)
    }

    pub fn stake_pools(&self) -> Result<Response, reqwest::Error> {
        self.get("stake_pools")
    }

    pub fn stake_distribution_at(&self, epoch: u32) -> Result<Response, reqwest::Error> {
        let request = format!("stake/{}", epoch);
        self.get(&request)
    }

    pub fn stats(&self) -> Result<Response, reqwest::Error> {
        self.get("node/stats")
    }

    pub fn network_stats(&self) -> Result<Response, reqwest::Error> {
        self.get("network/stats")
    }

    pub fn p2p_quarantined(&self) -> Result<Response, reqwest::Error> {
        self.get("network/p2p/quarantined")
    }

    pub fn p2p_non_public(&self) -> Result<Response, reqwest::Error> {
        self.get("network/p2p/non_public")
    }

    pub fn p2p_available(&self) -> Result<Response, reqwest::Error> {
        self.get("network/p2p/available")
    }

    pub fn p2p_view(&self) -> Result<Response, reqwest::Error> {
        self.get("network/p2p/view")
    }

    pub fn leaders_log(&self) -> Result<Response, reqwest::Error> {
        self.get("leaders/logs")
    }

    pub fn tip(&self) -> Result<Response, reqwest::Error> {
        self.get("tip")
    }

    pub fn settings(&self) -> Result<Response, reqwest::Error> {
        self.get("settings")
    }

    pub fn fragment_logs(&self) -> Result<Response, reqwest::Error> {
        self.get("fragment/logs")
    }

    pub fn leaders(&self) -> Result<Response, reqwest::Error> {
        self.get("leaders")
    }

    fn construct_headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            CONTENT_TYPE,
            HeaderValue::from_static("application/octet-stream"),
        );
        headers
    }

    fn post(
        &self,
        path: &str,
        body: Vec<u8>,
    ) -> Result<reqwest::blocking::Response, reqwest::Error> {
        self.client
            .post(&self.path(path, ApiVersion::V0))
            .headers(self.construct_headers())
            .body(body)
            .send()
    }

    pub fn send_fragment(&self, fragment: Fragment) -> Result<Response, reqwest::Error> {
        let raw = fragment.serialize_as_vec().unwrap();
        self.send_raw_fragment(raw)
    }

    pub fn send_raw_fragment(
        &self,
        body: Vec<u8>,
    ) -> Result<reqwest::blocking::Response, reqwest::Error> {
        self.post("message", body)
    }

    pub fn send_raw_fragments(&self, bodies: Vec<Vec<u8>>) -> Result<(), reqwest::Error> {
        let clients: Vec<reqwest::blocking::RequestBuilder> = bodies
            .into_iter()
            .map(|body| {
                self.client
                    .post(&self.path("message", ApiVersion::V0))
                    .headers(self.construct_headers())
                    .body(body)
            })
            .collect();

        for client in clients {
            client.send()?;
        }
        Ok(())
    }

    pub fn fragments_logs(&self) -> Result<Response, reqwest::Error> {
        self.client
            .get(&self.path("fragments/logs", ApiVersion::V1))
            .send()
    }

    pub fn fragments_statuses(&self, ids: Vec<String>) -> Result<Response, reqwest::Error> {
        self.client
            .get(&self.path("fragments/statuses", ApiVersion::V1))
            .query(&[("fragment_ids", ids.join(","))])
            .send()
    }

    pub fn send_fragment_batch(
        &self,
        fragments: Vec<Fragment>,
        fail_fast: bool,
    ) -> Result<Response, reqwest::Error> {
        self.client
            .post(&self.path("fragments", ApiVersion::V1))
            .headers(self.construct_headers())
            .json(&FragmentsBatch {
                fail_fast,
                fragments,
            })
            .send()
    }

    pub fn vote_plan_statuses(&self) -> Result<Response, reqwest::Error> {
        self.get("vote/active/plans")
    }

    pub fn send_until_ok<F>(&self, action: F, mut wait: Wait) -> Result<(), RestError>
    where
        F: Fn(&RawRest) -> Result<Response, reqwest::Error>,
    {
        loop {
            let response = action(&self);
            println!("Waiting for 200... {:?}", response);
            if let Ok(response) = response {
                if response.status().is_success() {
                    return Ok(());
                }
            }
            wait.check_timeout()?;
            wait.advance();
        }
    }
}
