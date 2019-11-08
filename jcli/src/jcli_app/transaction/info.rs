use crate::jcli_app::{
    transaction::{common, Error},
    utils::{io, OutputFormat},
};
use chain_addr::AddressReadable;
use chain_impl_mockchain::transaction::{Balance, UnspecifiedAccountIdentifier};
use jormungandr_lib::crypto::hash::Hash;
use jormungandr_lib::interfaces::TransactionInputType;
use serde_json::json;
use std::{io::Write, path::PathBuf};
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct Info {
    #[structopt(flatten)]
    common: common::CommonTransaction,

    #[structopt(flatten)]
    fee: common::CommonFees,

    /// write the info in the given file or print it to the standard output
    #[structopt(long = "output")]
    output: Option<PathBuf>,

    #[structopt(flatten)]
    output_format: OutputFormat,

    /// set the address prefix to use when displaying the addresses
    #[structopt(long = "prefix", default_value = "ca")]
    address_prefix: String,
}

impl Info {
    pub fn exec(self) -> Result<(), Error> {
        let staging = self.common.load()?;

        let inputs = staging
            .inputs()
            .iter()
            .map(|input| match input.input {
                TransactionInputType::Utxo(utxo_ptr, index) => Ok(json!({
                    "kind": "utxo",
                    "value": input.value,
                    "txid": Hash::from(utxo_ptr),
                    "index": index,
                })),
                TransactionInputType::Account(account) => {
                    let account_id = UnspecifiedAccountIdentifier::from(account)
                        .to_single_account()
                        .ok_or(Error::InfoExpectedSingleAccount)?;
                    Ok(json!({
                        "kind": "account",
                        "value": input.value,
                        "account": account_id.to_string(),
                    }))
                }
            })
            .collect::<Result<Vec<_>, Error>>()?;

        let outputs = staging.outputs().iter().map(|output| {
            json!({
                "address": AddressReadable::from_address(&self.address_prefix, output.address().as_ref()).to_string(),
                "value":  output.value(),
            })
        }).collect::<Vec<_>>();

        let fee_algo = self.fee.linear_fee();
        let balance = match staging.balance(&fee_algo)? {
            Balance::Negative(value) | Balance::Positive(value) => value.0,
            Balance::Zero => 0,
        };
        let info = json!({
            "status": staging.staging_kind_name(),
            "sign_data_hash": staging.transaction_sign_data_hash().to_string(),
            "num_inputs": staging.inputs().len(),
            "num_outputs": staging.outputs().len(),
            "num_witnesses": staging.witness_count(),
            "input": staging.total_input()?.0,
            "output": staging.total_output()?.0,
            "fee": staging.fees(&fee_algo).0,
            "balance": balance,
            "inputs": inputs,
            "outputs": outputs,
        });

        let mut output =
            io::open_file_write(&self.output).map_err(|source| Error::InfoFileWriteFailed {
                source,
                path: self.output.clone().unwrap_or_default(),
            })?;
        writeln!(output, "{}", self.output_format.format_json(info)?).map_err(|source| {
            Error::InfoFileWriteFailed {
                source,
                path: self.output.clone().unwrap_or_default(),
            }
        })?;

        Ok(())
    }
}
