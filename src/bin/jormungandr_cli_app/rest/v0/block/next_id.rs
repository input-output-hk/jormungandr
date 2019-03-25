use cardano::util::hex;
use chain_crypto::Blake2b256;
use jormungandr_cli_app::utils::HostAddr;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum NextId {
    /// Get block descendant ID
    Get {
        #[structopt(flatten)]
        addr: HostAddr,
        /// Maximum number of IDs, must be between 1 and 100, default 1
        #[structopt(short, long)]
        count: Option<usize>,
    },
}

impl NextId {
    pub fn exec(self, block_id: String) {
        match self {
            NextId::Get { addr, count } => exec_get(block_id, addr, count),
        }
    }
}

fn exec_get(block_id: String, addr: HostAddr, count: Option<usize>) {
    let url = addr
        .with_segments(&["v0", "block", &block_id, "next_id"])
        .into_url();
    let mut body = vec![];
    reqwest::Client::new()
        .get(url)
        .query(&[("count", count)])
        .send()
        .unwrap()
        .error_for_status()
        .unwrap()
        .copy_to(&mut body)
        .unwrap();
    for block_id in body.chunks(Blake2b256::HASH_SIZE) {
        println!("{}", hex::encode(block_id));
    }
}
