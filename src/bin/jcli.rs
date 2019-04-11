extern crate bech32;
extern crate cardano;
extern crate chain_addr;
extern crate chain_core;
extern crate chain_crypto;
extern crate chain_impl_mockchain;
extern crate jormungandr_utils;
extern crate num_traits;
extern crate rand;
extern crate rand_chacha;
extern crate reqwest;
extern crate serde;
extern crate serde_json;
extern crate serde_yaml;
extern crate structopt;
#[macro_use(custom_error)]
extern crate custom_error;
extern crate strfmt;

mod jcli_app;

use structopt::StructOpt;

fn main() {
    jcli_app::JCli::from_args().exec()
}
