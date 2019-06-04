use jcli_app::utils::{DebugFlag, HostAddr, RestApiSender};
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum Account {
    /// Get account state
    Get {
        #[structopt(flatten)]
        addr: HostAddr,
        #[structopt(flatten)]
        debug: DebugFlag,
        /// ID of an account, bech32-encoded
        account_id: String,
    },
}

impl Account {
    pub fn exec(self) {
        let (addr, debug, account_id) = match self {
            Account::Get {
                addr,
                debug,
                account_id,
            } => (addr, debug, account_id),
        };
        let url = addr
            .with_segments(&["v0", "account", &account_id])
            .unwrap()
            .into_url();
        let builder = reqwest::Client::new().get(url);
        let response = RestApiSender::new(builder, &debug).send().unwrap();
        response.response().error_for_status_ref().unwrap();
        let state: serde_json::Value = response.body().json().unwrap();
        let state_yaml = serde_yaml::to_string(&state).unwrap();
        println!("{}", state_yaml);
    }
}
