use jcli_app::rest::Error;
use jcli_app::utils::{DebugFlag, HostAddr, OutputFormat, RestApiSender};
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum Utxo {
    /// Get all UTXOs
    Get {
        #[structopt(flatten)]
        addr: HostAddr,
        #[structopt(flatten)]
        debug: DebugFlag,
        #[structopt(flatten)]
        output_format: OutputFormat,
    },
}

impl Utxo {
    pub fn exec(self) -> Result<(), Error> {
        let Utxo::Get {
            addr,
            debug,
            output_format,
        } = self;
        let url = addr.with_segments(&["v0", "utxo"])?.into_url();
        let builder = reqwest::Client::new().get(url);
        let response = RestApiSender::new(builder, &debug).send()?;
        response.response().error_for_status_ref()?;
        let status = response.body().json_value()?;
        let formatted = output_format.format_json(status)?;
        println!("{}", formatted);
        Ok(())
    }
}
