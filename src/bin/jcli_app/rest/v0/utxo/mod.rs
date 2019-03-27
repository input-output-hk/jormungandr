use jcli_app::utils::HostAddr;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum Utxo {
    /// Get all UTXOs
    Get {
        #[structopt(flatten)]
        addr: HostAddr,
    },
}

impl Utxo {
    pub fn exec(self) {
        let addr = match self {
            Utxo::Get { addr } => addr,
        };
        let url = addr.with_segments(&["v0", "utxo"]).into_url();
        let utxos: serde_json::Value = reqwest::Client::new()
            .get(url)
            .send()
            .unwrap()
            .error_for_status()
            .unwrap()
            .json()
            .unwrap();
        let utxos_yaml = serde_yaml::to_string(&utxos).unwrap();
        println!("{}", utxos_yaml);
    }
}
