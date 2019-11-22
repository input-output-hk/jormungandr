use crate::jcli_app::rest::Error;
use crate::jcli_app::utils::{DebugFlag, HostAddr, OutputFormat, RestApiSender};
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct Utxo {
    /// hex-encoded ID of the transaction fragment
    fragment_id: String,

    /// index of the transaction output
    output_index: u8,

    #[structopt(subcommand)]
    subcommand: Subcommand,
}

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
enum Subcommand {
    /// Get UTxO details
    Get {
        #[structopt(flatten)]
        output_format: OutputFormat,

        #[structopt(flatten)]
        addr: HostAddr,

        #[structopt(flatten)]
        debug: DebugFlag,
    },
}

impl Utxo {
    pub fn exec(self) -> Result<(), Error> {
        let Subcommand::Get {
            output_format,
            addr,
            debug,
        } = self.subcommand;
        let url = addr
            .with_segments(&[
                "v0",
                "utxo",
                &self.fragment_id,
                &self.output_index.to_string(),
            ])?
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
