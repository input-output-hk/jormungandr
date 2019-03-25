use super::next_id::NextId;
use cardano::util::hex;
use jormungandr_cli_app::utils::HostAddr;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum Subcommand {
    /// Get block
    Get {
        #[structopt(flatten)]
        addr: HostAddr,
    },
    /// Get block descendant ID
    NextId(NextId),
}

impl Subcommand {
    pub fn exec(self, block_id: String) {
        match self {
            Subcommand::Get { addr } => exec_get(block_id, addr),
            Subcommand::NextId(next_id) => next_id.exec(block_id),
        }
    }
}

fn exec_get(block_id: String, addr: HostAddr) {
    let url = addr.with_segments(&["v0", "block", &block_id]).into_url();
    let mut body = vec![];
    reqwest::Client::new()
        .get(url)
        .send()
        .unwrap()
        .error_for_status()
        .unwrap()
        .copy_to(&mut body)
        .unwrap();
    println!("{}", hex::encode(&body));
}
