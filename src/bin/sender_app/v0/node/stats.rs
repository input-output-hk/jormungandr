use reqwest::Client;
use serde_json::Value;
use structopt::StructOpt;
use utils::HostAddr;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum Stats {
    /// Get node information
    Get {
        #[structopt(flatten)]
        addr: HostAddr,
    },
}

impl Stats {
    pub fn exec(self) {
        let addr = match self {
            Stats::Get { addr } => addr,
        };
        let url = addr.with_segments(&["v0", "node", "stats"]).into_url();
        let status: Value = Client::new()
            .get(url)
            .send()
            .unwrap()
            .error_for_status()
            .unwrap()
            .json()
            .unwrap();
        println!("{:#}", status);
    }
}
