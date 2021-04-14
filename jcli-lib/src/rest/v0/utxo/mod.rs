use crate::rest::{Error, RestArgs};
use crate::utils::OutputFormat;
#[cfg(feature = "structopt")]
use structopt::StructOpt;

#[cfg_attr(
    feature = "structopt",
    derive(StructOpt),
    structopt(rename_all = "kebab-case")
)]
pub struct Utxo {
    /// hex-encoded ID of the transaction fragment
    fragment_id: String,

    /// index of the transaction output
    output_index: u8,

    #[cfg_attr(feature = "structopt", structopt(subcommand))]
    subcommand: Subcommand,
}

#[cfg_attr(
    feature = "structopt",
    derive(StructOpt),
    structopt(rename_all = "kebab-case")
)]
enum Subcommand {
    /// Get UTxO details
    Get {
        #[cfg_attr(feature = "structopt", structopt(flatten))]
        output_format: OutputFormat,

        #[cfg_attr(feature = "structopt", structopt(flatten))]
        args: RestArgs,
    },
}

impl Utxo {
    pub fn exec(self) -> Result<(), Error> {
        let Subcommand::Get {
            args,
            output_format,
        } = self.subcommand;
        let response = args
            .client()?
            .get(&[
                "v0",
                "utxo",
                &self.fragment_id,
                &self.output_index.to_string(),
            ])
            .execute()?
            .json()?;
        let formatted = output_format.format_json(response)?;
        println!("{}", formatted);
        Ok(())
    }
}
