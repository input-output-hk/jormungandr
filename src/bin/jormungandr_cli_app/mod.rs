mod address;
mod block;
mod rest;
mod transaction;
mod utils;

use structopt::StructOpt;

/// Jormungandr CLI toolkit
#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum JormungandrCli {
    /// Address tooling and helper
    Address(address::Address),
    /// Block tooling and helper
    Block(block::Block),
    /// Send request to node REST API
    Rest(rest::Rest),
    /// Build and view offline transaction
    Transaction(transaction::Transaction),
}

impl JormungandrCli {
    pub fn exec(self) {
        match self {
            JormungandrCli::Address(address) => address.exec(),
            JormungandrCli::Block(block) => block.exec(),
            JormungandrCli::Rest(rest) => rest.exec(),
            JormungandrCli::Transaction(transaction) => transaction.exec(),
        }
    }
}
