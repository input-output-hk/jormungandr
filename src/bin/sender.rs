extern crate cardano;
extern crate chain_addr;
extern crate chain_core;
extern crate chain_crypto;
extern crate chain_impl_mockchain;
extern crate reqwest;
extern crate serde_json;
extern crate structopt;

mod sender_app;
mod utils;

use structopt::StructOpt;

fn main() {
    sender_app::SenderApp::from_args().exec();
}
