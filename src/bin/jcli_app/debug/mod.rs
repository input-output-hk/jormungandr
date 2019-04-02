mod message;

use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum Debug {
    /// Decode hex-encoded message an display its content
    Message(message::Message),
}

impl Debug {
    pub fn exec(self) {
        match self {
            Debug::Message(message) => message.exec(),
        }
    }
}
