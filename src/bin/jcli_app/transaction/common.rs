use chain_impl_mockchain::fee::LinearFee;
use jcli_app::transaction::staging::{Staging, StagingError};
use std::{io::Read, path::PathBuf};
use structopt::StructOpt;

type StaticStr = &'static str;

custom_error! {pub CommonError
    Io { source: std::io::Error } = "I/O Error",
    Bech32Parse { source: bech32::Error } = "Invalid Bech32",
    CannotParse { source: chain_core::mempack::ReadError } = "Invalid formatted transaction",
    HrpInvalid { expected: StaticStr, actual: String } = "Invalid transaction HRP: it reads `{actual}' but was expecting `{expected}'",
    TooManyWitnesses = "Authenticated Transaction has too many witnesses compared to number of inputs",
    NotATransactionMessage = "Not a block message for transaction",
}

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct CommonFees {
    #[structopt(long = "fee-constant", default_value = "0")]
    pub constant: u64,
    #[structopt(long = "fee-coefficient", default_value = "0")]
    pub coefficient: u64,
    #[structopt(long = "fee-certificate", default_value = "0")]
    pub certificate: u64,
}

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct CommonTransaction {
    /// place where the transaction is going to be save during its staging phase
    /// If a file is given, the transaction will be read from this file and
    /// modification will be written into this same file.
    /// If no file is given, the transaction will be read from the standard
    /// input and will be rendered in the standard output
    #[structopt(long = "staging", alias = "transaction")]
    pub staging_file: Option<PathBuf>,
}

impl CommonFees {
    pub fn linear_fee(&self) -> LinearFee {
        LinearFee::new(self.constant, self.coefficient, self.certificate)
    }
}

impl CommonTransaction {
    pub fn load(&self) -> Result<Staging, StagingError> {
        Staging::load(&self.staging_file)
    }

    pub fn store(&self, staging: &Staging) -> Result<(), StagingError> {
        staging.store(&self.staging_file)
    }
}

pub fn read_bytes<R: Read>(mut reader: R, hrp: &'static str) -> Result<Vec<u8>, CommonError> {
    use bech32::{Bech32, FromBase32 as _};

    let mut bech32_encoded_transaction = String::new();
    reader.read_to_string(&mut bech32_encoded_transaction)?;

    let bech32_encoded_transaction: Bech32 = bech32_encoded_transaction.trim_end().parse()?;
    if bech32_encoded_transaction.hrp() != hrp {
        return Err(CommonError::HrpInvalid {
            expected: hrp,
            actual: bech32_encoded_transaction.hrp().to_string(),
        });
    }
    Ok(Vec::from_base32(bech32_encoded_transaction.data())?)
}
