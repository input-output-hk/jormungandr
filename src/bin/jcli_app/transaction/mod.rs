mod build;

use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum Transaction {
    /// Build transaction and write it to stdout as hex-encoded message
    Build(build::Build),
}

impl Transaction {
    pub fn exec(self) {
        match self {
            Transaction::Build(build) => build.exec(),
        }
    }
}
