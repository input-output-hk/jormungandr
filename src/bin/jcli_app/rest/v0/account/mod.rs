use jcli_app::utils::HostAddr;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum Account {
    /// Get account state
    Get {
        #[structopt(flatten)]
        addr: HostAddr,
        /// ID of an account, bech32-encoded
        account_id: String,
    },
}

impl Account {
    pub fn exec(self) {
        let (addr, account_id) = match self {
            Account::Get { addr, account_id } => (addr, account_id),
        };
        let url = addr
            .with_segments(&["v0", "account", &account_id])
            .unwrap()
            .into_url();
        let state: serde_json::Value = reqwest::Client::new()
            .get(url)
            .send()
            .unwrap()
            .error_for_status()
            .unwrap()
            .json()
            .unwrap();
        let state_yaml = serde_yaml::to_string(&state).unwrap();
        println!("{}", state_yaml);
    }
}
