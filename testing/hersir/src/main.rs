mod args;
mod config;
mod error;
mod spawn;

use args::Args;
use jormungandr_testing_utils::testing::adversary::{process::AdversaryNodeBuilder, rest::start};
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use structopt::StructOpt;

fn main() {
    let args = Args::from_args();

    let nodes = match spawn::spawn_network(&args) {
        Ok(nodes) => nodes,
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    };

    if args.adversary {
        if args.verbose {
            eprint!("Launching adversary on 127.0.0.1:18210");
        }

        let genesis_block = nodes
            .values()
            .next()
            .unwrap()
            .block0_configuration()
            .to_block();

        let adversary = Arc::new(Mutex::new(
            AdversaryNodeBuilder::new(genesis_block)
                // .with_server_enabled()
                .with_alias(String::from("Baddie"))
                .build(),
        ));

        start(adversary);
    }

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
