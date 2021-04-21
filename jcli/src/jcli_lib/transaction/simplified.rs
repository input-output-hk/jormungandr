use crate::block::{load_block, open_block_file};
use crate::jcli_lib::rest::RestArgs;
use crate::jcli_lib::transaction::{common, Error};
use crate::transaction::mk_witness::WitnessType;
use crate::transaction::staging::Staging;
use crate::utils::key_parser::read_ed25519_secret_key_from_file;
use crate::utils::AccountId;
use crate::{rest, transaction};
use chain_impl_mockchain::account::SpendingCounter;
use chain_impl_mockchain::key::EitherEd25519SecretKey;
use chain_impl_mockchain::transaction::Output;
use jormungandr_lib::interfaces;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct SimplifiedTransaction {
    /// the file path to the file to read the signing key from.
    /// If omitted it will be read from the standard input.
    pub secret: Option<PathBuf>,

    /// the account to debit the funds from
    #[structopt(name = "ACCOUNT")]
    pub faucet_address: interfaces::Address,

    /// the UTxO address or account address to credit funds to
    #[structopt(name = "ADDRESS")]
    pub receiver_address: interfaces::Address,

    /// the value
    #[structopt(name = "VALUE")]
    pub value: interfaces::Value,

    #[structopt(flatten)]
    pub common: common::CommonTransaction,

    #[structopt(flatten)]
    pub fee: common::CommonFees,

    /// Set the change in the given address
    pub change: Option<interfaces::Address>,

    #[structopt(default_value = "block-0.bin")]
    pub block0_path: PathBuf,

    #[structopt(flatten)]
    rest_args: RestArgs,
}

impl SimplifiedTransaction {
    pub fn exec(self) -> Result<(), Error> {
        let secret_key = read_ed25519_secret_key_from_file(&self.secret)?;
        simplified_transaction(
            self.faucet_address,
            self.receiver_address,
            secret_key,
            self.value,
            self.fee,
            self.block0_path,
            self.rest_args,
            self.change,
        )?;
        Ok(())
    }
}

pub fn simplified_transaction(
    faucet_address: interfaces::Address,
    receiver_address: interfaces::Address,
    secret_key: EitherEd25519SecretKey,
    value: interfaces::Value,
    fee: common::CommonFees,
    block0_file: PathBuf,
    rest_args: RestArgs,
    change: Option<interfaces::Address>,
) -> Result<(), Error> {
    let mut transaction = Staging::new();

    // add account
    transaction::add_account::add_account(faucet_address.clone(), value, &mut transaction)?;

    // add output
    transaction.add_output(Output {
        address: receiver_address.into(),
        value: value.into(),
    })?;

    //finalize
    transaction::finalize::finalize(fee, change, &mut transaction)?;

    // get transaction and block0 ids
    let transaction_sign_data_hash = transaction.transaction_sign_data_hash();
    let block0 = load_block(open_block_file(&Some(block0_file))?)?;
    let block0_hash = block0.header.id();

    // get spending counter
    let account_state = rest::v0::account::request_account_information(
        rest_args,
        AccountId::try_from_str(&faucet_address.to_string())?,
    )?;

    //make witness
    let witness = transaction::mk_witness::make_witness(
        &WitnessType::Account,
        &block0_hash,
        &transaction_sign_data_hash,
        Some(SpendingCounter::from(account_state.counter())),
        &secret_key,
    )?;

    Ok(())
}
