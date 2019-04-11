use chain_impl_mockchain::message::Message;
use jcli_app::transaction::common;
use structopt::StructOpt;

custom_error! {pub FinalizeError
    Io { source: std::io::Error } = "I/O error",
    Bech32 { source: bech32::Error } = "Invalid Bech32",
    WriteMessage { error: common::CommonError } = "cannot write the finalized transaction: {error}",
    ReadTransaction { error: common::CommonError } = "cannot read the transaction: {error}",
    NotEnoughWitnesses = "Not enough witnesses, cannot finalize the transaction",
}

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct Finalize {
    #[structopt(flatten)]
    pub common: common::CommonTransaction,
}

impl Finalize {
    pub fn exec(self) -> Result<(), FinalizeError> {
        let transaction = self
            .common
            .load_auth_transaction()
            .map_err(|error| FinalizeError::ReadTransaction { error })?;

        if transaction.witnesses.len() != transaction.transaction.inputs.len() {
            Err(FinalizeError::NotEnoughWitnesses)
        } else {
            self.common
                .write_message(&Message::Transaction(transaction))
                .map_err(|error| FinalizeError::WriteMessage { error })
        }
    }
}
