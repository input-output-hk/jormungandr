extern crate bech32;
extern crate cardano;
extern crate chain_addr;
extern crate chain_core;
extern crate chain_crypto;
extern crate chain_impl_mockchain;
extern crate reqwest;
extern crate serde_json;
extern crate structopt;

mod jormungandr_cli_app;

use structopt::StructOpt;

fn main() {
    jormungandr_cli_app::JormungandrCli::from_args().exec()
}
