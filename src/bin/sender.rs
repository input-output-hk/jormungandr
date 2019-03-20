extern crate reqwest;
extern crate serde_json;
extern crate structopt;

mod sender_app;

use structopt::StructOpt;

fn main() {
    sender_app::SenderApp::from_args().exec();
}
