mod args;
mod config;
mod error;
mod spawn;

use args::Args;
use std::time::Duration;
use structopt::StructOpt;

fn main() {
    let args = Args::from_args();

    if let Err(e) = spawn::spawn_network(args) {
        eprintln!("{}", e);
        std::process::exit(1);
    };

    loop {
        std::thread::sleep(Duration::from_secs(1));
    }
}
