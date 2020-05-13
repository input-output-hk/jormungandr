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
        let request = format!("http://{}/api/v0/{}", self.endpoint, path);
        reqwest::blocking::get(&request)
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

    pub fn fragment_logs(&self) -> Result<String, reqwest::Error> {
        self.get("fragment/logs")?.text()
    }

    pub fn leaders(&self) -> Result<String, reqwest::Error> {
        self.get("leaders")?.text()
    }
}
