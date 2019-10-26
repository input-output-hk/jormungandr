use crate::{fragment::Fragment, testing::builders::tx_builder, transaction::*};
use chain_addr::Address;
use std::vec::Vec;

pub fn create_initial_transaction(output: Output<Address>) -> Fragment {
    let mut builder = tx_builder::TransactionBuilder::new();
    let authenticator = builder.with_output(output).authenticate();
    authenticator.as_message()
}

pub fn create_initial_transactions(outputs: &Vec<Output<Address>>) -> Fragment {
    let mut builder = tx_builder::TransactionBuilder::new();
    let authenticator = builder.with_outputs(outputs.to_vec()).authenticate();
    authenticator.as_message()
}
