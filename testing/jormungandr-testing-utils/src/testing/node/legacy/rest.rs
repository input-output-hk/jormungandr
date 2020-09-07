use crate::{
    testing::{node::RestError, MemPoolCheck},
    wallet::Wallet,
};
use bech32::FromBase32;
use chain_crypto::PublicKey;
use chain_impl_mockchain::account;
use chain_impl_mockchain::fragment::{Fragment, FragmentId};
use jormungandr_lib::{crypto::hash::Hash, interfaces::FragmentLog};
use reqwest::{
    blocking::Response,
    header::{HeaderMap, HeaderValue, CONTENT_TYPE},
};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Settings {
    pub enable_debug: bool,
    pub use_https_for_post: bool,
    pub certificate: Option<reqwest::Certificate>,
}

impl Settings {
    pub fn new_use_https_for_post() -> Settings {
        Settings {
            enable_debug: false,
            use_https_for_post: true,
            certificate: None,
        }
    }
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            enable_debug: false,
            use_https_for_post: false,
            certificate: None,
        }
    }
}

/// Legacy tolerant rest api
/// This layer returns raw strings without deserialization
/// in order to assure compatibility and lack of serde errors
#[derive(Debug, Clone)]
pub struct BackwardCompatibleRest {
    uri: String,
    settings: Settings,
}

impl BackwardCompatibleRest {
    pub fn new(uri: String, settings: Settings) -> Self {
        Self { uri, settings }
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

    fn print_request_path(&self, text: &str) {
        if self.settings.enable_debug {
            println!("Request: {}", text);
        }
    }

    pub fn disable_logger(&mut self) {
        self.settings.enable_debug = false;
    }

    pub fn enable_logger(&mut self) {
        self.settings.enable_debug = true;
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
        self.print_request_path(&request);
        match &self.settings.certificate {
            None => reqwest::blocking::get(&request),
            Some(cert) => {
                let client = reqwest::blocking::Client::builder()
                    .use_rustls_tls()
                    .add_root_certificate(cert.clone())
                    .build()
                    .unwrap();
                client.get(&request).send()
            }
        }
    }

    fn path(&self, path: &str) -> String {
        format!("{}/v0/{}", self.uri, path)
    }

    fn path_http_or_https(&self, path: &str) -> String {
        if self.settings.use_https_for_post {
            let url = url::Url::parse(&self.uri).unwrap();
            return format!(
                "https://{}:443/{}/v0/{}",
                url.domain().unwrap(),
                url.path_segments().unwrap().next().unwrap(),
                path
            );
        }
        format!("{}/v0/{}", self.uri, path)
    }

    pub fn stake_distribution(&self) -> Result<String, reqwest::Error> {
        let response_text = self.get("stake")?.text()?;
        self.print_response_text(&response_text);
        Ok(response_text)
    }

    pub fn account_state(&self, wallet: &Wallet) -> Result<String, reqwest::Error> {
        self.account_state_by_pk(&wallet.identifier().to_bech32_str())
    }

    pub fn account_state_by_pk(&self, bech32_str: &str) -> Result<String, reqwest::Error> {
        let key = hex::encode(Self::try_from_str(bech32_str).as_ref().as_ref());
        let request = format!("account/{}", key);
        let response_text = self.get(&request)?.text()?;
        self.print_response_text(&response_text);
        Ok(response_text)
    }

    fn try_from_str(src: &str) -> account::Identifier {
        let (_, data) = bech32::decode(src).unwrap();
        let dat = Vec::from_base32(&data).unwrap();
        let pk = PublicKey::from_binary(&dat).unwrap();
        account::Identifier::from(pk)
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

    pub fn tip(&self) -> Result<Hash, RestError> {
        let tip = self.get("tip")?.text()?;
        tip.parse().map_err(RestError::HashParseError)
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
        let builder = reqwest::blocking::Client::builder();
        let client = builder.build()?;
        client
            .post(&self.path_http_or_https(path))
            .headers(self.construct_headers())
            .body(body)
            .send()
    }

    pub fn send_fragment(&self, fragment: Fragment) -> Result<MemPoolCheck, reqwest::Error> {
        use chain_core::property::Fragment as _;
        use chain_core::property::Serialize as _;

        let raw = fragment.serialize_as_vec().unwrap();
        let fragment_id = fragment.id();
        let response = self.send_raw_fragment(raw)?;
        self.print_response_text(&response.text()?);

        Ok(MemPoolCheck::new(fragment_id))
    }

    pub fn send_raw_fragment(
        &self,
        body: Vec<u8>,
    ) -> Result<reqwest::blocking::Response, reqwest::Error> {
        let response = self.post("message", body)?;
        self.print_debug_response(&response);
        Ok(response)
    }

    pub fn vote_plan_statuses(&self) -> Result<String, reqwest::Error> {
        self.get("vote/active/plans")?.text()
    }
}
