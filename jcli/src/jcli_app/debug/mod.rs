mod message;

use hex::FromHexError;
use jcli_app::utils::error::CustomErrorFiller;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum Debug {
    /// Decode hex-encoded message an display its content
    Message(message::Message),
}

custom_error! {pub Error
    Io { source: std::io::Error } = "I/O Error",
    InputInvalid { source: std::io::Error, path: PathBuf }
        = @{{ let _ = source; format_args!("invalid input file path '{}'", path.display()) }},
    HexMalformed { source: FromHexError } = "hex encoding malformed",
    MessageMalformed { source: std::io::Error, filler: CustomErrorFiller } = "message malformed",
}

impl Debug {
    pub fn exec(self) -> Result<(), Error> {
        match self {
            Debug::Message(message) => message.exec(),
        }
    }
}
