mod args;
mod config;
mod error;
mod spawn;

use args::Args;
use std::time::Duration;
use structopt::StructOpt;

fn main() {
    let args = Args::from_args();

    let _processes = match spawn::spawn_network(args) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    };

    loop {
        std::thread::sleep(Duration::from_secs(1));
    }
}
