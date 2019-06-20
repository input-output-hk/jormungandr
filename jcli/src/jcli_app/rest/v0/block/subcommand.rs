use super::next_id::NextId;
use hex;
use jcli_app::utils::{DebugFlag, HostAddr, RestApiSender};
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum Subcommand {
    /// Get block
    Get {
        #[structopt(flatten)]
        addr: HostAddr,
        #[structopt(flatten)]
        debug: DebugFlag,
    },
    /// Get block descendant ID
    NextId(NextId),
}

impl Subcommand {
    pub fn exec(self, block_id: String) {
        match self {
            Subcommand::Get { addr, debug } => exec_get(block_id, addr, debug),
            Subcommand::NextId(next_id) => next_id.exec(block_id),
        }
    }
}

fn exec_get(block_id: String, addr: HostAddr, debug: DebugFlag) {
    let url = addr
        .with_segments(&["v0", "block", &block_id])
        .unwrap()
        .into_url();
    let builder = reqwest::Client::new().get(url);
    let response = RestApiSender::new(builder, &debug).send().unwrap();
    response.response().error_for_status_ref().unwrap();
    let body = response.body().binary();
    println!("{}", hex::encode(&body));
}
