use crate::jcli_lib::{
    rest::{Error, RestArgs},
    utils::OutputFormat,
};
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum StakePool {
    /// Get stake pool details
    Get {
        #[structopt(flatten)]
        args: RestArgs,
        /// hex-encoded pool ID
        pool_id: String,
        #[structopt(flatten)]
        output_format: OutputFormat,
    },
}

impl StakePool {
    pub fn exec(self) -> Result<(), Error> {
        let StakePool::Get {
            args,
            pool_id,
            output_format,
        } = self;
        let response = args
            .client()?
            .get(&["v0", "stake_pool", &pool_id])
            .execute()?
            .json()?;
        let formatted = output_format.format_json(response)?;
        println!("{}", formatted);
        Ok(())
    }
}
