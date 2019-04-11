use bech32::{Bech32, FromBase32 as _, ToBase32 as _};
use chain_addr::Address;
use chain_core::{
    mempack::{ReadBuf, Readable as _},
    property::{Deserialize as _, Serialize as _},
};
use chain_impl_mockchain::{
    fee::LinearFee,
    message::Message,
    transaction::{AuthenticatedTransaction, NoExtra, Transaction},
};
use jcli_app::utils::io;
use std::{
    io::{Read, Write},
    path::PathBuf,
};
use structopt::StructOpt;

type StaticStr = &'static str;

const TRANSACTION_HRP: StaticStr = "tx";
const AUTH_TRANSACTION_HRP: StaticStr = "authtx";
const MESSAGE_HRP: StaticStr = "message";

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
    /// modification will be read from this same file.
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
    pub fn load_transaction(&self) -> Result<Transaction<Address, NoExtra>, CommonError> {
        let reader = io::open_file_read(&self.staging_file);
        read_transaction(reader)
    }

    pub fn write_transaction(
        &self,
        transaction: &Transaction<Address, NoExtra>,
    ) -> Result<(), CommonError> {
        let writer = io::open_file_write(&self.staging_file);
        write_transaction(writer, transaction)
    }
    pub fn load_auth_transaction(
        &self,
    ) -> Result<AuthenticatedTransaction<Address, NoExtra>, CommonError> {
        let reader = io::open_file_read(&self.staging_file);
        let auth = read_auth_transaction(reader)?;

        let input_len = auth.transaction.inputs.len();
        let witness_len = auth.witnesses.len();
        if witness_len > input_len {
            Err(CommonError::TooManyWitnesses)
        } else {
            Ok(auth)
        }
    }

    pub fn write_auth_transaction(
        &self,
        transaction: &AuthenticatedTransaction<Address, NoExtra>,
    ) -> Result<(), CommonError> {
        let writer = io::open_file_write(&self.staging_file);
        write_auth_transaction(writer, transaction)
    }

    pub fn load_message(&self) -> Result<AuthenticatedTransaction<Address, NoExtra>, CommonError> {
        let reader = io::open_file_read(&self.staging_file);
        let auth = match read_message(reader)? {
            Message::Transaction(auth) => auth,
            _ => return Err(CommonError::NotATransactionMessage),
        };

        let input_len = auth.transaction.inputs.len();
        let witness_len = auth.witnesses.len();
        if witness_len > input_len {
            Err(CommonError::TooManyWitnesses)
        } else {
            Ok(auth)
        }
    }

    pub fn write_message(&self, message: &Message) -> Result<(), CommonError> {
        let writer = io::open_file_write(&self.staging_file);
        write_message(writer, message)
    }
}

pub fn read_bytes<R: Read>(mut reader: R, hrp: &str) -> Result<Vec<u8>, CommonError> {
    let mut bech32_encoded_transaction = String::new();
    reader.read_to_string(&mut bech32_encoded_transaction)?;

    let bech32_encoded_transaction: Bech32 = bech32_encoded_transaction.trim_end().parse()?;
    if bech32_encoded_transaction.hrp() != hrp {
        return Err(CommonError::HrpInvalid {
            expected: TRANSACTION_HRP,
            actual: bech32_encoded_transaction.hrp().to_string(),
        });
    }
    Ok(Vec::from_base32(bech32_encoded_transaction.data())?)
}

fn read_transaction<R: Read>(reader: R) -> Result<Transaction<Address, NoExtra>, CommonError> {
    let bytes = read_bytes(reader, TRANSACTION_HRP)?;

    let mut buf = ReadBuf::from(&bytes);
    let transaction = Transaction::read(&mut buf)?;
    Ok(transaction)
}

fn write_transaction<W: Write>(
    mut writer: W,
    transaction: &Transaction<Address, NoExtra>,
) -> Result<(), CommonError> {
    let bytes = transaction.serialize_as_vec()?;

    let base32 = bytes.to_base32();
    let bech32 = Bech32::new(TRANSACTION_HRP.to_owned(), base32)?;
    writeln!(writer, "{}", bech32)?;
    Ok(())
}

fn read_auth_transaction<R: Read>(
    reader: R,
) -> Result<AuthenticatedTransaction<Address, NoExtra>, CommonError> {
    let bytes = read_bytes(reader, AUTH_TRANSACTION_HRP)?;

    let mut buf = ReadBuf::from(&bytes);
    let transaction = AuthenticatedTransaction::read(&mut buf)?;
    Ok(transaction)
}

fn write_auth_transaction<W: Write>(
    mut writer: W,
    transaction: &AuthenticatedTransaction<Address, NoExtra>,
) -> Result<(), CommonError> {
    let bytes = transaction.serialize_as_vec()?;

    let base32 = bytes.to_base32();
    let bech32 = Bech32::new(AUTH_TRANSACTION_HRP.to_owned(), base32)?;
    writeln!(writer, "{}", bech32)?;
    Ok(())
}

fn read_message<R: Read>(reader: R) -> Result<Message, CommonError> {
    let bytes = read_bytes(reader, MESSAGE_HRP)?;

    let mut buf = std::io::BufReader::new(std::io::Cursor::new(bytes));
    let transaction = Message::deserialize(&mut buf)?;
    Ok(transaction)
}

fn write_message<W: Write>(mut writer: W, message: &Message) -> Result<(), CommonError> {
    let bytes = message.serialize_as_vec()?;

    let base32 = bytes.to_base32();
    let bech32 = Bech32::new(MESSAGE_HRP.to_owned(), base32)?;
    writeln!(writer, "{}", bech32)?;
    Ok(())
}
