use hex;
use jcli_app::rest::Error;
use jcli_app::utils::{io, DebugFlag, HostAddr, OutputFormat, RestApiSender};
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum Message {
    /// Post message
    Post {
        #[structopt(flatten)]
        addr: HostAddr,
        #[structopt(flatten)]
        debug: DebugFlag,
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
        addr: HostAddr,
        #[structopt(flatten)]
        debug: DebugFlag,
        #[structopt(flatten)]
        output_format: OutputFormat,
    },
}

impl Message {
    pub fn exec(self) -> Result<(), Error> {
        match self {
            Message::Post { addr, debug, file } => post_message(file, addr, debug),
            Message::Logs {
                addr,
                debug,
                output_format,
            } => get_logs(addr, debug, output_format),
        }
    }
}

fn get_logs(addr: HostAddr, debug: DebugFlag, output_format: OutputFormat) -> Result<(), Error> {
    let url = addr.with_segments(&["v0", "fragment", "logs"])?.into_url();
    let builder = reqwest::Client::new().get(url);
    let response = RestApiSender::new(builder, &debug).send()?;
    response.response().error_for_status_ref()?;
    let status = response.body().json_value()?;
    let formatted = output_format.format_json(status)?;
    println!("{}", formatted);
    Ok(())
}

fn post_message(file: Option<PathBuf>, addr: HostAddr, debug: DebugFlag) -> Result<(), Error> {
    let msg_hex = io::read_line(&file)?;
    let msg_bin = hex::decode(&msg_hex)?;
    let url = addr.with_segments(&["v0", "message"])?.into_url();
    let builder = reqwest::Client::new().post(url);
    let response = RestApiSender::new(builder, &debug)
        .with_binary_body(msg_bin)
        .send()?;
    response.response().error_for_status_ref()?;
    println!("Success!");
    Ok(())
}
