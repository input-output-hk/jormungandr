use chain_crypto::Blake2b256;
use jcli_app::rest::Error;
use jcli_app::utils::{DebugFlag, HostAddr, RestApiSender};
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum NextId {
    /// Get block descendant ID
    Get {
        #[structopt(flatten)]
        addr: HostAddr,
        #[structopt(flatten)]
        debug: DebugFlag,
        /// Maximum number of IDs, must be between 1 and 100, default 1
        #[structopt(short, long)]
        count: Option<usize>,
    },
}

impl NextId {
    pub fn exec(self, block_id: String) -> Result<(), Error> {
        match self {
            NextId::Get { addr, debug, count } => exec_get(block_id, addr, debug, count),
        }
    }
}

fn exec_get(
    block_id: String,
    addr: HostAddr,
    debug: DebugFlag,
    count: Option<usize>,
) -> Result<(), Error> {
    let url = addr
        .with_segments(&["v0", "block", &block_id, "next_id"])?
        .into_url();
    let builder = reqwest::Client::new().get(url).query(&[("count", count)]);
    let response = RestApiSender::new(builder, &debug).send()?;
    response.response().error_for_status_ref()?;
    let body = response.body().binary();
    for block_id in body.chunks(Blake2b256::HASH_SIZE) {
        println!("{}", hex::encode(block_id));
    }
    Ok(())
}
