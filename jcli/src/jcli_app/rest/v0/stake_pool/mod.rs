use crate::jcli_app::rest::Error;
use crate::jcli_app::utils::{DebugFlag, HostAddr, OutputFormat, RestApiSender};
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum StakePool {
    /// Get stake pool details
    Get {
        /// hex-encoded pool ID
        pool_id: String,
        #[structopt(flatten)]
        addr: HostAddr,
        #[structopt(flatten)]
        debug: DebugFlag,
        #[structopt(flatten)]
        output_format: OutputFormat,
    },
}

impl StakePool {
    pub fn exec(self) -> Result<(), Error> {
        let StakePool::Get {
            pool_id,
            addr,
            debug,
            output_format,
        } = self;
        let url = addr
            .with_segments(&["v0", "stake_pool", &pool_id])?
            .into_url();
        let builder = reqwest::Client::new().get(url);
        let response = RestApiSender::new(builder, &debug).send()?;
        response.ok_response()?;
        let status = response.body().json_value()?;
        let formatted = output_format.format_json(status)?;
        println!("{}", formatted);
        Ok(())
    }
}
