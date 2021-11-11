use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt)]
pub struct Args {
    #[structopt(long, short)]
    pub config: PathBuf,

    #[structopt(long, short)]
    pub adversary: bool,

    #[structopt(long, short)]
    pub verbose: bool,
}
