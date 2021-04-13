use crate::rest::{Error, RestArgs};
use chain_crypto::Blake2b256;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum NextId {
    /// Get block descendant ID
    Get {
        #[structopt(flatten)]
        args: RestArgs,
        /// Maximum number of IDs, must be between 1 and 100, default 1
        #[structopt(short, long)]
        count: Option<usize>,
    },
}

impl NextId {
    pub fn exec(self, block_id: String) -> Result<(), Error> {
        match self {
            NextId::Get { args, count } => exec_get(args, block_id, count),
        }
    }
}

fn exec_get(args: RestArgs, block_id: String, count: Option<usize>) -> Result<(), Error> {
    let response = args
        .client()?
        .get(&["v0", "block", &block_id, "next_id"])
        .query(&[("count", count)])
        .execute()?
        .bytes()?;
    for block_id in response.chunks(Blake2b256::HASH_SIZE) {
        println!("{}", hex::encode(block_id));
    }
    Ok(())
}
