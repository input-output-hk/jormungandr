#[macro_use]
extern crate clap;
#[macro_use]
extern crate serde_derive;
extern crate serde_yaml;
#[macro_use]
extern crate log;
extern crate env_logger;

extern crate cardano;
extern crate cardano_storage;
extern crate exe_common;

use std::path::{PathBuf};

pub struct State {
}

impl State {
    pub fn new() -> Self {
        State {}
    }
}

fn main() {
    /// load parameters & config
    let mut state = State::new();

    /// bootstrap phase (peer discovery, download all the existing blocks)

    /// connect to peers
    /// core logic
    println!("Hello, world!");
}
