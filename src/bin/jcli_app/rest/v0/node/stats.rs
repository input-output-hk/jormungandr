use jcli_app::utils::HostAddr;
use structopt::StructOpt;

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
        let status: serde_json::Value = reqwest::Client::new()
            .get(url)
            .send()
            .unwrap()
            .error_for_status()
            .unwrap()
            .json()
            .unwrap();
        let status_yaml = serde_yaml::to_string(&status).unwrap();
        println!("{}", status_yaml);
    }
}
