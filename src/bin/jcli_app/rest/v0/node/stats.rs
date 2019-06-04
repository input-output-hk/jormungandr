use jcli_app::utils::{DebugFlag, HostAddr, RestApiSender};
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum Stats {
    /// Get node information
    Get {
        #[structopt(flatten)]
        addr: HostAddr,
        #[structopt(flatten)]
        debug: DebugFlag,
    },
}

impl Stats {
    pub fn exec(self) {
        let (addr, debug) = match self {
            Stats::Get { addr, debug } => (addr, debug),
        };
        let url = addr
            .with_segments(&["v0", "node", "stats"])
            .unwrap()
            .into_url();
        let builder = reqwest::Client::new().get(url);
        let response = RestApiSender::new(builder, &debug).send().unwrap();
        response.response().error_for_status_ref().unwrap();
        let status: serde_json::Value = response.body().json().unwrap();
        let status_yaml = serde_yaml::to_string(&status).unwrap();
        println!("{}", status_yaml);
    }
}
