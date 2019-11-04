use chain_addr::{Address, AddressReadable};
use chain_impl_mockchain::transaction::{AccountIdentifier, Balance, Output};
use jcli_app::{
    transaction::{common, staging::Staging, Error},
    utils::io,
};
use jormungandr_lib::crypto::hash::Hash;
use jormungandr_lib::interfaces::{TransactionInput, TransactionInputType, TransactionOutput};
use std::{collections::HashMap, io::Write, path::PathBuf};
use strfmt::strfmt;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct Info {
    #[structopt(flatten)]
    pub common: common::CommonTransaction,

    #[structopt(flatten)]
    pub fee: common::CommonFees,

    /// write the info in the given file or print it to the standard output
    #[structopt(long = "output")]
    pub output: Option<PathBuf>,

    /// formatting for the output to displays
    /// user "{name}" to display the variable with the named `name'.
    ///
    /// available variables: sign-data-hash, num_inputs, num_outputs, num_witnesses, fee
    /// balance, input, output and status
    ///
    #[structopt(
        long = "format",
        default_value = "Transaction {sign-data-hash} ({status})\n  Input:   {input}\n  Output:  {output}\n  Fees:    {fee}\n  Balance: {balance}\n"
    )]
    pub format: String,

    /// display only the inputs of type UTxO
    #[structopt(long = "only-utxos")]
    pub only_utxos: bool,
    /// display only the inputs of type Account
    #[structopt(long = "only-accounts")]
    pub only_accounts: bool,
    /// display only the outputs
    #[structopt(long = "only-outputs")]
    pub only_outputs: bool,

    /// formatting for the UTxO inputs of the transaction. This format
    /// will be applied to every inputs of type UTxO.
    ///
    /// available variables: txid, index and value.
    ///
    #[structopt(
        long = "format-utxo-input",
        alias = "utxo",
        default_value = " - {txid}:{index} {value}\n"
    )]
    pub format_utxo_input: String,

    /// formatting for the Account inputs of the transaction. This format
    /// will be applied to every inputs of type account.
    ///
    /// available variables: account and value.
    ///
    #[structopt(
        long = "format-account-input",
        alias = "account",
        default_value = " - {account} {value}\n"
    )]
    pub format_account_input: String,

    /// Display the outputs of the transaction, this function will be called
    /// for every outputs of the transaction
    ///
    /// available variables: address and value.
    #[structopt(
        long = "format-output",
        alias = "output",
        default_value = " + {address} {value}\n"
    )]
    pub format_output: String,

    /// set the address prefix to use when displaying the addresses
    #[structopt(long = "prefix", default_value = "ca")]
    address_prefix: String,
}

impl Info {
    pub fn exec(self) -> Result<(), Error> {
        let transaction = self.common.load()?;

        let mut output =
            io::open_file_write(&self.output).map_err(|source| Error::InfoFileWriteFailed {
                source,
                path: self.output.clone().unwrap_or_default(),
            })?;

        self.display_info(&mut output, &transaction)?;
        self.display_inputs(&mut output, &transaction.inputs())?;

        if !self.only_accounts || !self.only_utxos {
            self.display_outputs(&mut output, transaction.outputs())?;
        }
        Ok(())
    }

    fn display_outputs(
        &self,
        mut writer: impl Write,
        outputs: &[TransactionOutput],
    ) -> Result<(), Error> {
        for output in outputs {
            self.display_output(&mut writer, output)?;
        }
        Ok(())
    }

    fn display_inputs(
        &self,
        mut writer: impl Write,
        inputs: &[TransactionInput],
    ) -> Result<(), Error> {
        for input in inputs {
            match input.input {
                TransactionInputType::Account(_) => {
                    if self.only_outputs || self.only_utxos {
                        continue;
                    }
                    self.display_input(&mut writer, input)?;
                }
                TransactionInputType::Utxo(..) => {
                    if self.only_outputs || self.only_accounts {
                        continue;
                    }
                    self.display_input(&mut writer, input)?;
                }
            }
        }
        Ok(())
    }

    fn display_output(&self, writer: impl Write, output: &TransactionOutput) -> Result<(), Error> {
        let mut vars = HashMap::new();

        let output: Output<Address> = output.clone().into();

        vars.insert(
            "address".to_owned(),
            AddressReadable::from_address(&self.address_prefix, &output.address).to_string(),
        );
        vars.insert("value".to_owned(), output.value.0.to_string());
        self.write_info(writer, &self.format_output, vars)
    }

    fn display_input(&self, writer: impl Write, input: &TransactionInput) -> Result<(), Error> {
        let mut vars = HashMap::new();
        match input.input {
            TransactionInputType::Utxo(utxo_ptr, index) => {
                vars.insert("txid".to_owned(), Hash::from(utxo_ptr).to_string());
                vars.insert("index".to_owned(), index.to_string());
                vars.insert("value".to_owned(), input.value.to_string());
                self.write_info(writer, &self.format_utxo_input, vars)
            }
            TransactionInputType::Account(account_id) => {
                let account_id: AccountIdentifier = account_id.into();
                let account: chain_crypto::PublicKey<_> = account_id
                    .to_single_account()
                    .ok_or(Error::InfoExpectedSingleAccount)?
                    .into();
                vars.insert("account".to_owned(), account.to_string());
                vars.insert("value".to_owned(), input.value.to_string());
                self.write_info(writer, &self.format_account_input, vars)
            }
        }
    }

    fn display_info(&self, writer: impl Write, transaction: &Staging) -> Result<(), Error> {
        let mut vars = HashMap::new();
        vars.insert("status".to_owned(), transaction.staging_kind_name());
        vars.insert(
            "sign-data-hash".to_owned(),
            transaction.transaction_sign_data_hash().to_string(),
        );
        vars.insert(
            "num_inputs".to_owned(),
            transaction.inputs().len().to_string(),
        );
        vars.insert(
            "num_outputs".to_owned(),
            transaction.outputs().len().to_string(),
        );
        vars.insert(
            "num_witnesses".to_owned(),
            transaction.witness_count().to_string(),
        );
        vars.insert("input".to_owned(), transaction.total_input()?.0.to_string());
        vars.insert(
            "output".to_owned(),
            transaction.total_output()?.0.to_string(),
        );
        let fee_algo = self.fee.linear_fee();
        vars.insert("fee".to_owned(), transaction.fees(&fee_algo).0.to_string());
        vars.insert(
            "balance".to_owned(),
            match transaction.balance(&fee_algo)? {
                Balance::Negative(value) => format!("-{}", value.0),
                Balance::Positive(value) => format!("+{}", value.0),
                Balance::Zero => "0".to_string(),
            },
        );
        self.write_info(writer, &self.format, vars)
    }

    fn write_info(
        &self,
        mut writer: impl Write,
        format: &str,
        info: HashMap<String, String>,
    ) -> Result<(), Error> {
        let formatted = strfmt(format, &info).map_err(|source| Error::InfoOutputFormatInvalid {
            source,
            format: format.to_string(),
        })?;
        write!(writer, "{}", formatted).map_err(|source| Error::InfoFileWriteFailed {
            source,
            path: self.output.clone().unwrap_or_default(),
        })
    }
}
