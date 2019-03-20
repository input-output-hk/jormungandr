mod utils;
mod v0;

use self::v0::V0;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum SenderApp {
    /// API version 0
    V0(V0),
}

impl SenderApp {
    pub fn exec(self) {
        match self {
            SenderApp::V0(v0) => v0.exec(),
        }
    }
}
