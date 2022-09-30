use std::path::PathBuf;
use structopt::StructOpt;

///
/// Hersir is a command line tool that lets you deploy a network of Jormungandr nodes
///
#[derive(StructOpt)]
pub struct Args {
    /// Path to config file
    #[structopt(long, short)]
    pub config: PathBuf,

    /// Enable verbose mode
    #[structopt(long, short)]
    pub verbose: bool,
}
