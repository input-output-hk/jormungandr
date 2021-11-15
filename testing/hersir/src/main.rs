mod args;
mod config;
mod error;
mod spawn;

use args::Args;
use std::time::Duration;
use structopt::StructOpt;

fn main() {
    let args = Args::from_args();

    let nodes = match spawn::spawn_network(args) {
        Ok(nodes) => nodes,
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    };

    loop {
        for node in nodes.values() {
            if let Err(e) = node.rest().network_stats() {
                eprintln!("{}", e);
                std::process::exit(1);
            }
        }
        std::thread::sleep(Duration::from_secs(1));
    }
}
