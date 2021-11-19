use hersir::{args::Args, spawn};
use structopt::StructOpt;

fn main() {
    let args = Args::from_args();
    if let Err(e) = spawn::spawn_network(args) {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}
