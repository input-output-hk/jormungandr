use cardano::util::hex;
use jcli_app::utils::HostAddr;
use reqwest::header::CONTENT_TYPE;
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
    },
}

impl Message {
    pub fn exec(self) {
        match self {
            Message::Post { addr, file } => post_message(file, addr),
            Message::Logs { addr } => get_logs(addr),
        }
    }
}

fn get_logs(addr: HostAddr) {
    let url = addr
        .with_segments(&["v0", "fragment", "logs"])
        .unwrap()
        .into_url();
    let logs: serde_json::Value = reqwest::Client::new()
        .get(url)
        .send()
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .unwrap();
    let logs_yaml = serde_yaml::to_string(&logs).unwrap();
    println!("{}", logs_yaml);
}

fn post_message(file: Option<PathBuf>, addr: HostAddr) {
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
    reqwest::Client::new()
        .post(url)
        .header(CONTENT_TYPE, "application/octet-stream")
        .body(msg_bin)
        .send()
        .unwrap()
        .error_for_status()
        .unwrap();
    println!("Success!");
}
