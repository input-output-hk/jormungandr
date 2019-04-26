use bech32::{Bech32, ToBase32 as _};
use chain_core::property::Serialize as _;
use chain_crypto::bech32::{self as bech32_crypto, Bech32 as _};
use chain_crypto::{AsymmetricKey, SecretKey, SecretKeyError};
use chain_impl_mockchain::{
    account::SpendingCounter,
    transaction::{TransactionId, Witness},
};
use jcli_app::{transaction::common, utils::io};
use std::{io::Read, path::PathBuf};
use structopt::StructOpt;

custom_error! {pub MkWitnessError
    Bech32 { source: bech32::Error } = "Invalid Bech32",
    Bech32Crypto { source: bech32_crypto::Error } = "Invalid Bech32",
    ReadTransaction { error: common::CommonError } = "cannot read the transaction: {error}",
    WriteTransaction { error: common::CommonError } = "cannot save changes of the transaction: {error}",
    Io { source: std::io::Error} = "cannot read or write data",
    SecretKey { source: SecretKeyError } = "Invalid secret key",
    MissingSpendingCounter = "parameter `--account-spending-counter' is mandatory when creating a witness for an account"
}

custom_error! {pub ParseWitnessTypeError
    Invalid = "Invalid witness type, expected `utxo', `legacy-utxo' or `account'"

}

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct MkWitness {
    /// the Transaction ID of the witness to sign
    #[structopt(name = "TRANSACTION_ID")]
    pub transaction_id: TransactionId,

    /// the file path to the file to write the witness in.
    /// If omitted it will be printed to the standard output.
    pub output: Option<PathBuf>,

    #[structopt(long = "type", parse(try_from_str))]
    pub witness_type: WitnessType,

    /// value is mandatory is `--type=account' It is the counter for
    /// every time the account is being utilized.
    pub account_spending_counter: Option<u32>,

    /// the file path to the file to read the signing key from.
    /// If omitted it will be read from the standard input.
    pub secret: Option<PathBuf>,
}

pub enum WitnessType {
    UTxO,
    OldUTxO,
    Account,
}
impl std::fmt::Display for WitnessType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            WitnessType::UTxO => write!(f, "utxo"),
            WitnessType::OldUTxO => write!(f, "legacy-utxo"),
            WitnessType::Account => write!(f, "account"),
        }
    }
}
impl std::str::FromStr for WitnessType {
    type Err = ParseWitnessTypeError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "utxo" => Ok(WitnessType::UTxO),
            "legacy-utxo" => Ok(WitnessType::OldUTxO),
            "account" => Ok(WitnessType::Account),
            _ => Err(ParseWitnessTypeError::Invalid),
        }
    }
}

impl MkWitness {
    fn secret<A: AsymmetricKey>(&self) -> Result<SecretKey<A>, MkWitnessError> {
        let mut bech32_str = String::new();
        io::open_file_read(&self.secret).read_to_string(&mut bech32_str)?;
        Ok(SecretKey::try_from_bech32_str(&bech32_str)?)
    }

    pub fn exec(self) -> Result<(), MkWitnessError> {
        let witness = match self.witness_type {
            WitnessType::UTxO => {
                let secret_key = self.secret()?;
                Witness::new_utxo(&self.transaction_id, &secret_key)
            }
            WitnessType::OldUTxO => {
                // let secret_key = self.secret()?;
                unimplemented!()
            }
            WitnessType::Account => {
                let account_spending_counter = self
                    .account_spending_counter
                    .ok_or(MkWitnessError::MissingSpendingCounter)
                    .map(SpendingCounter::from)?;

                let secret_key = self.secret()?;
                Witness::new_account(&self.transaction_id, &account_spending_counter, &secret_key)
            }
        };

        self.write_witness(&witness)
    }

    fn write_witness(&self, witness: &Witness) -> Result<(), MkWitnessError> {
        let mut writer = io::open_file_write(&self.output);
        let bytes = witness.serialize_as_vec()?;

        let base32 = bytes.to_base32();
        let bech32 = Bech32::new("witness".to_owned(), base32)?;
        writeln!(writer, "{}", bech32)?;
        Ok(())
    }
}
