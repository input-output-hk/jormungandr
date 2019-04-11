mod add_account;
mod add_input;
mod add_output;
mod add_witness;
mod common;
mod finalize;
mod info;
mod lock;
mod mk_witness;
mod new;

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
    Lock(lock::Lock),
    /// Finalize the transaction
    Finalize(finalize::Finalize),
    /// get the Transaction ID from the given transaction
    /// (if the transaction is edited, the returned value will change)
    Id(common::CommonTransaction),
    /// display the info regarding a given transaction
    Info(info::Info),
    /// create witnesses
    MakeWitness(mk_witness::MkWitness),
}

custom_error! {pub TransactionError
    NewError { source: new::NewError } = "Cannot create new transaction",
    AddInputError { source: add_input::AddInputError } = "Cannot add input to the transaction",
    AddAccountError { source: add_account::AddAccountError } = "Cannot add input account to the transaction",
    AddOutputError { source: add_output::AddOutputError } = "Cannot add output to the transaction",
    AddWitnessError { source: add_witness::AddWitnessError } = "Cannot add witness to the transaction",
    InfoError { source: info::InfoError } = "{source}",
    TransactionError { source: common::CommonError } = "Invalid transaction",
    LockError { source: lock::LockError } = "cannot lock transaction",
    FinalizeError { source: finalize::FinalizeError } = "cannot finalize transaction",
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
            Transaction::Lock(lock) => lock.exec()?,
            Transaction::Finalize(finalize) => finalize.exec()?,
            Transaction::Id(common) => display_id(common)?,
            Transaction::Info(info) => info.exec()?,
            Transaction::MakeWitness(mk_witness) => mk_witness.exec()?,
        }

        Ok(())
    }
}

fn display_id(common: common::CommonTransaction) -> Result<(), TransactionError> {
    let id = common.load_transaction().map(|tx| tx.hash()).or_else(|_| {
        common
            .load_auth_transaction()
            .map(|auth_tx| auth_tx.transaction.hash())
    })?;

    println!("{}", id);
    Ok(())
}
