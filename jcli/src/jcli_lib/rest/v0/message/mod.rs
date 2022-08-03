use crate::jcli_lib::{
    rest::{Error, RestArgs},
    utils::{io, OutputFormat},
};
use chain_core::{
    packer::Codec,
    property::{DeserializeFromSlice as _, Serialize},
};
use chain_impl_mockchain::fragment::Fragment;
use serde_json::Value;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum Message {
    /// Post message. Prints id for posted message
    Post {
        #[structopt(flatten)]
        args: RestArgs,
        /// File containing hex-encoded message.
        /// If not provided, message will be read from stdin.
        #[structopt(short, long)]
        file: Option<PathBuf>,
    },

    /// get the node's logs on the message pool. This will provide information
    /// on pending transaction, rejected transaction and or when a transaction
    /// has been added in a block
    Logs {
        #[structopt(flatten)]
        args: RestArgs,
        #[structopt(flatten)]
        output_format: OutputFormat,
    },
}

impl Message {
    pub fn exec(self) -> Result<(), Error> {
        match self {
            Message::Post { args, file } => post_message(args, file),
            Message::Logs {
                args,
                output_format,
            } => get_logs(args, output_format),
        }
    }
}

fn get_logs(args: RestArgs, output_format: OutputFormat) -> Result<(), Error> {
    let response = args
        .client()?
        .get(&["v0", "fragment", "logs"])
        .execute()?
        .json()?;
    let formatted = output_format.format_json(response)?;
    println!("{}", formatted);
    Ok(())
}

fn post_message(args: RestArgs, file: Option<PathBuf>) -> Result<(), Error> {
    let msg_hex = io::read_line(&file)?;
    let msg_bin = hex::decode(&msg_hex)?;
    let fragment = Fragment::deserialize_from_slice(&mut Codec::new(msg_bin.as_slice()))?;
    let fragment_id = post_fragment(args, fragment)?;
    println!("{}", fragment_id);
    Ok(())
}

pub fn post_fragment(args: RestArgs, fragment: Fragment) -> Result<Value, Error> {
    let fragment_ids = args
        .client()?
        .post(&["v0", "message"])
        .body(fragment.serialize_as_vec()?)
        .execute()?
        .text()?;
    // the response comes back as base64 encoded json
    let json_bytes = base64::decode(&fragment_ids)?;
    let json_str = String::from_utf8(json_bytes)?;
    Ok(serde_json::from_str(&json_str)?)
}
