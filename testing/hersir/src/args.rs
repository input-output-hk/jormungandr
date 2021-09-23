use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt)]
pub struct Args {
    #[structopt(long = "config", short = "c")]
    pub config: PathBuf,
}
