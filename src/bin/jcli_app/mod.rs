mod address;
mod block;
mod rest;
mod transaction;
mod utils;

use structopt::StructOpt;

/// Jormungandr CLI toolkit
#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum JCli {
    /// Address tooling and helper
    Address(address::Address),
    /// Block tooling and helper
    Block(block::Block),
    /// Send request to node REST API
    Rest(rest::Rest),
    /// Build and view offline transaction
    Transaction(transaction::Transaction),
}

impl JCli {
    pub fn exec(self) {
        match self {
            JCli::Address(address) => address.exec(),
            JCli::Block(block) => block.exec(),
            JCli::Rest(rest) => rest.exec(),
            JCli::Transaction(transaction) => transaction.exec(),
        }
    }
}
