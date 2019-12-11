mod block;
mod message;
use crate::jcli_app::utils::error::CustomErrorFiller;
use hex::FromHexError;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum Debug {
    /// Decode hex-encoded message and display its content
    Message(message::Message),
    /// Decode hex-encoded block and display its content
    Block(block::Block),
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
            Debug::Block(block) => block.exec(),
        }
    }
}
