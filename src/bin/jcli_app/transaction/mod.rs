mod add_account;
mod add_input;
mod add_output;
mod add_witness;
mod common;
mod finalize;
mod info;
mod mk_witness;
mod new;
mod seal;
mod staging;

use cardano::util::hex;
use chain_core::property::Serialize as _;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum Transaction {
    /// create a new staging transaction. The transaction is initially
    /// empty.
    New(new::New),

    /// add UTxO input to the transaction
    AddInput(add_input::AddInput),
    /// add Account input to the transaction
    AddAccount(add_account::AddAccount),
    /// add output to the transaction
    AddOutput(add_output::AddOutput),
    /// add output to the finalized transaction
    AddWitness(add_witness::AddWitness),
    /// Lock a transaction and start adding witnesses
    Finalize(finalize::Finalize),
    /// Finalize the transaction
    Seal(seal::Seal),
    /// get the Transaction ID from the given transaction
    /// (if the transaction is edited, the returned value will change)
    Id(common::CommonTransaction),
    /// display the info regarding a given transaction
    Info(info::Info),
    /// create witnesses
    MakeWitness(mk_witness::MkWitness),
    /// get the message format out of a sealed transaction
    ToMessage(common::CommonTransaction),
}

custom_error! {pub TransactionError
    NewError { source: new::NewError } = "Cannot create new transaction",
    AddInputError { error: add_input::AddInputError } = "{error}",
    AddAccountError { source: add_account::AddAccountError } = "Cannot add input account to the transaction",
    AddOutputError { source: add_output::AddOutputError } = "Cannot add output to the transaction",
    AddWitnessError { source: add_witness::AddWitnessError } = "Cannot add witness to the transaction",
    InfoError { source: info::InfoError } = "{source}",
    TransactionError { source: common::CommonError } = "Invalid transaction",
    FinalizeError { source: finalize::FinalizeError } = "cannot finalize transaction",
    SealError { source: seal::SealError } = "cannot seal transaction",
    MakeWitness { source: mk_witness::MkWitnessError } = "Cannot make witness",
}

impl Transaction {
    pub fn exec(self) -> Result<(), TransactionError> {
        match self {
            Transaction::New(new) => new.exec()?,
            Transaction::AddInput(add_input) => add_input.exec()?,
            Transaction::AddAccount(add_account) => add_account.exec()?,
            Transaction::AddOutput(add_output) => add_output.exec()?,
            Transaction::AddWitness(add_witness) => add_witness.exec()?,
            Transaction::Finalize(finalize) => finalize.exec()?,
            Transaction::Seal(seal) => seal.exec()?,
            Transaction::Id(common) => display_id(common)?,
            Transaction::Info(info) => info.exec()?,
            Transaction::MakeWitness(mk_witness) => mk_witness.exec()?,
            Transaction::ToMessage(common) => display_message(common)?,
        }

        Ok(())
    }
}

fn display_id(common: common::CommonTransaction) -> Result<(), TransactionError> {
    let id = common.load()?.transaction().hash();

    println!("{}", id);
    Ok(())
}

fn display_message(common: common::CommonTransaction) -> Result<(), TransactionError> {
    let message = common.load()?.message()?;

    let bytes: Vec<u8> = message.serialize_as_vec().unwrap();

    println!("{}", hex::encode(&bytes));
    Ok(())
}
