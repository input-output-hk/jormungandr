mod v0;

use structopt::StructOpt;

/// Send request to node REST API
#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum Rest {
    /// API version 0
    V0(v0::V0),
}

impl Rest {
    pub fn exec(self) {
        match self {
            Rest::V0(v0) => v0.exec(),
        }
    }
}
