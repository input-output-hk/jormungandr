use chain_core::property::Deserialize as _;
use chain_impl_mockchain::fragment::Fragment as MockFragment;
use hex;
use jcli_app::debug::Error;
use jcli_app::utils::{error::CustomErrorFiller, io};
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt)]
pub struct Message {
    /// file containing hex-encoded message. If not provided, it will be read from stdin.
    #[structopt(short, long)]
    input: Option<PathBuf>,
}

impl Message {
    pub fn exec(self) -> Result<(), Error> {
        let reader = io::open_file_read(&self.input).map_err(|source| Error::InputInvalid {
            source,
            path: self.input.unwrap_or_default(),
        })?;
        let mut hex_str = String::new();
        BufReader::new(reader).read_line(&mut hex_str)?;
        let bytes = hex::decode(hex_str.trim())?;
        let message = MockFragment::deserialize(bytes.as_ref()).map_err(|source| {
            Error::MessageMalformed {
                source,
                filler: CustomErrorFiller,
            }
        })?;
        println!("{:#?}", message);
        Ok(())
    }
}
