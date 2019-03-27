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
}

impl Message {
    pub fn exec(self) {
        let (addr, file) = match self {
            Message::Post { addr, file } => (addr, file),
        };
        let msg_hex = match file {
            Some(path) => fs::read_to_string(path).unwrap(),
            None => {
                let stdin = stdin();
                let mut lines = stdin.lock().lines();
                lines.next().unwrap().unwrap()
            }
        };
        let msg_bin = hex::decode(msg_hex.trim()).unwrap();
        let url = addr.with_segments(&["v0", "message"]).into_url();
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
}
