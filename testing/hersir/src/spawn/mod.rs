mod interactive;
mod monitor;
mod standard;

use crate::{
    args::Args,
    config::{Config, SessionMode},
    error::Error,
};
use std::fs::File;

pub fn spawn_network(args: Args) -> Result<(), Error> {
    let config: Config = serde_yaml::from_reader(File::open(&args.config)?)?;

    match &config.session.mode {
        SessionMode::Standard => standard::spawn_network(config, args),
        SessionMode::Monitor => monitor::spawn_network(config, args),
        SessionMode::Interactive => interactive::spawn_network(config),
    }
}
