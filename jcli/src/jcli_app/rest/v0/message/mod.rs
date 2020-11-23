use crate::jcli_app::{
    rest::Error,
    utils::{io, DebugFlag, HostAddr, OutputFormat, RestApiSender, TlsCert},
};
use chain_core::property::Deserialize;
use chain_impl_mockchain::fragment::Fragment;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum Message {
    /// Post message. Prints id for posted message
    Post {
        #[structopt(flatten)]
        addr: HostAddr,
        #[structopt(flatten)]
        debug: DebugFlag,
        /// File containing hex-encoded message.
        /// If not provided, message will be read from stdin.
        #[structopt(short, long)]
        file: Option<PathBuf>,
        #[structopt(flatten)]
        tls: TlsCert,
    },

    /// get the node's logs on the message pool. This will provide information
    /// on pending transaction, rejected transaction and or when a transaction
    /// has been added in a block
    Logs {
        #[structopt(flatten)]
        addr: HostAddr,
        #[structopt(flatten)]
        debug: DebugFlag,
        #[structopt(flatten)]
        output_format: OutputFormat,
        #[structopt(flatten)]
        tls: TlsCert,
    },
}

impl Message {
    pub fn exec(self) -> Result<(), Error> {
        match self {
            Message::Post {
                addr,
                debug,
                tls,
                file,
            } => post_message(file, addr, debug, tls),
            Message::Logs {
                addr,
                debug,
                output_format,
                tls,
            } => get_logs(addr, debug, tls, output_format),
        }
    }
}

fn get_logs(
    addr: HostAddr,
    debug: DebugFlag,
    tls: TlsCert,
    output_format: OutputFormat,
) -> Result<(), Error> {
    let url = addr.with_segments(&["v0", "fragment", "logs"])?.into_url();
    let builder = reqwest::blocking::Client::new().get(url);
    let response = RestApiSender::new(builder, &debug, &tls).send()?;
    response.ok_response()?;
    let status = response.body().json_value()?;
    let formatted = output_format.format_json(status)?;
    println!("{}", formatted);
    Ok(())
}

fn post_message(
    file: Option<PathBuf>,
    addr: HostAddr,
    debug: DebugFlag,
    tls: TlsCert,
) -> Result<(), Error> {
    let msg_hex = io::read_line(&file)?;
    let msg_bin = hex::decode(&msg_hex)?;
    let _fragment =
        Fragment::deserialize(msg_bin.as_slice()).map_err(Error::InputFragmentMalformed)?;
    let url = addr.with_segments(&["v0", "message"])?.into_url();
    let builder = reqwest::blocking::Client::new().post(url);
    let response = RestApiSender::new(builder, &debug, &tls)
        .with_binary_body(msg_bin)
        .send()?;
    response.ok_response()?;
    let fragment_id = response.body().text();
    println!("{}", fragment_id.as_ref());
    Ok(())
}
