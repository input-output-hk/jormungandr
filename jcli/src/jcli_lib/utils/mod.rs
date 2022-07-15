pub mod account_id;
pub mod io;
pub mod key_parser;
pub mod output_file;
pub mod output_format;
pub mod vote;

pub use self::{account_id::AccountId, output_format::OutputFormat};
use structopt::StructOpt;
use thiserror::Error;

#[derive(StructOpt)]
#[structopt(name = "utils", rename_all = "kebab-case")]
pub enum Utils {
    /// convert a bech32 with hrp n into a bech32 with prefix m
    Bech32Convert(Bech32ConvertArgs),
}

#[derive(StructOpt)]
pub struct Bech32ConvertArgs {
    /// the bech32 you want to convert
    #[structopt(name = "FROM_BECH32")]
    from_bech32: String,

    /// the new bech32 hrp you want to use
    #[structopt(name = "NEW_PREFIX")]
    new_hrp: String,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("failed to convert bech32")]
    Bech32ConversionFailure(#[from] bech32::Error),
}

impl Utils {
    pub fn exec(self) -> Result<(), Error> {
        match self {
            Utils::Bech32Convert(convert_args) => {
                convert_prefix(convert_args.from_bech32, convert_args.new_hrp)
            }
        }
    }
}

fn convert_prefix(from_addr: String, prefix: String) -> Result<(), Error> {
    let (_, d, variant) = bech32::decode(&from_addr)?;
    let n = bech32::encode(&prefix, d, variant)?;
    println!("{}", n);
    Ok(())
}
