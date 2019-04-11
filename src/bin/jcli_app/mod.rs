mod address;
mod block;
mod certificate;
mod debug;
mod key;
mod rest;
mod transaction;
mod utils;

use structopt::StructOpt;

/// Jormungandr CLI toolkit
#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum JCli {
    /// Key Generation
    Key(key::Key),
    /// Address tooling and helper
    Address(address::Address),
    /// Block tooling and helper
    Genesis(block::Genesis),
    /// Send request to node REST API
    Rest(rest::Rest),
    /// Build and view offline transaction
    Transaction(transaction::Transaction),
    /// Debug tools for developers
    Debug(debug::Debug),
    /// Certificate generation tool
    Certificate(certificate::Certificate),
}

impl JCli {
    pub fn exec(self) {
        match self {
            JCli::Key(key) => key.exec(),
            JCli::Address(address) => address.exec(),
            JCli::Genesis(genesis) => genesis.exec(),
            JCli::Rest(rest) => rest.exec(),
            JCli::Transaction(transaction) => {
                if let Err(error) = transaction.exec() {
                    report_error(error)
                }
            }
            JCli::Debug(debug) => debug.exec(),
            JCli::Certificate(certificate) => certificate.exec(),
        }
    }
}

fn report_error<E: std::error::Error>(error: E) {
    eprintln!("{}", error);
    let mut source = error.source();
    while let Some(sub_error) = source {
        eprintln!("  |-> {}", sub_error);
        source = sub_error.source();
    }

    std::process::exit(1)
}
