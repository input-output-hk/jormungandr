mod interactive;
mod monitor;
mod standard;

use crate::config::{Config, SessionMode};
use crate::{args::Args, error::Error};
use std::fs::File;

pub fn spawn_network(args: Args) -> Result<(), Error> {
    let config: Config = serde_yaml::from_reader(File::open(&args.config)?)?;
    let topology = config.build_topology();

    match &config.session.mode {
        SessionMode::Standard => standard::spawn_network(config, topology, args),
        SessionMode::Monitor => monitor::spawn_network(config, topology, args),
        SessionMode::Interactive => interactive::spawn_network(config, topology),
    }
}
