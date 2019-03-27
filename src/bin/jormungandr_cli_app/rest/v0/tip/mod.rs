use jormungandr_cli_app::utils::HostAddr;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum Tip {
    /// Get tip ID
    Get {
        #[structopt(flatten)]
        addr: HostAddr,
    },
}

impl Tip {
    pub fn exec(self) {
        let addr = match self {
            Tip::Get { addr } => addr,
        };
        let url = addr.with_segments(&["v0", "tip"]).into_url();
        let tip = reqwest::Client::new()
            .get(url)
            .send()
            .unwrap()
            .error_for_status()
            .unwrap()
            .text()
            .unwrap();
        println!("{}", tip);
    }
}
