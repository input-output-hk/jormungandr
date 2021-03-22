use jcli_lib;

use std::error::Error;
use structopt::StructOpt;

fn main() {
    jcli_lib::JCli::from_args()
        .exec()
        .unwrap_or_else(report_error)
}

fn report_error(error: Box<dyn Error>) {
    eprintln!("{}", error);
    let mut source = error.source();
    while let Some(sub_error) = source {
        eprintln!("  |-> {}", sub_error);
        source = sub_error.source();
    }
    std::process::exit(1)
}
