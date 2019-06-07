use cardano::util::hex;
use jcli_app::utils::{DebugFlag, HostAddr, OutputFormat, RestApiSender};
use std::fs;
use std::io::{stdin, BufRead};
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
    pub fn exec(self) {
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

fn get_logs(addr: HostAddr, debug: DebugFlag, output_format: OutputFormat) {
    let url = addr
        .with_segments(&["v0", "fragment", "logs"])
        .unwrap()
        .into_url();
    let builder = reqwest::Client::new().get(url);
    let response = RestApiSender::new(builder, &debug).send().unwrap();
    response.response().error_for_status_ref().unwrap();
    let status = response.body().json_value().unwrap();
    let formatted = output_format.format_json(status).unwrap();
    println!("{}", formatted);
}

fn post_message(file: Option<PathBuf>, addr: HostAddr, debug: DebugFlag) {
    let msg_hex = match file {
        Some(path) => fs::read_to_string(path).unwrap(),
        None => {
            let stdin = stdin();
            let mut lines = stdin.lock().lines();
            lines.next().unwrap().unwrap()
        }
    };
    let msg_bin = hex::decode(msg_hex.trim()).unwrap();
    let url = addr.with_segments(&["v0", "message"]).unwrap().into_url();
    let builder = reqwest::Client::new().post(url);
    let response = RestApiSender::new(builder, &debug)
        .with_binary_body(msg_bin)
        .send()
        .unwrap();
    response.response().error_for_status_ref().unwrap();
    println!("Success!");
}
