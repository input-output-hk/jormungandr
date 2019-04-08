use self::tx_address_readable::TxAddressReadable;
use self::tx_input::TxInput;
use self::tx_output::TxOutput;
use chain_addr::Address;
use chain_core::property::Serialize as _;
use chain_crypto::bech32::Bech32 as _;
use chain_crypto::Signature;
use chain_impl_mockchain::fee::LinearFee;
use chain_impl_mockchain::message::Message;
use chain_impl_mockchain::transaction::{AuthenticatedTransaction, NoExtra, Transaction, Witness};
use chain_impl_mockchain::txbuilder::{OutputPolicy, TransactionBuilder};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use structopt::StructOpt;

mod tx_address_readable;
mod tx_input;
mod tx_output;

#[derive(Debug, Default, Deserialize, Serialize, StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct TxData {
    /// transaction input. Must have format
    /// `<hex-encoded-transaction-id>:<output-index>:<value>`.
    /// E.g. `1234567890abcdef:2:535`. At least 1 value required.
    #[structopt(name = "input", short, long)]
    inputs: Vec<TxInput>,
    /// transaction output. Must have format `<address>:<value>`.
    /// E.g. `ed25519extended_public1abcdef1234567890:501`.
    /// The address must be bech32-encoded ed25519extended_public key.
    /// At least 1 value required.
    #[structopt(name = "output", short, long)]
    outputs: Vec<TxOutput>,
    /// change address. Value taken from inputs and not spent on outputs or fees will be
    /// returned to this address. If not provided, the change will go to treasury.
    /// Must be bech32-encoded ed25519extended_public key.
    #[structopt(short, long)]
    change: Option<TxAddressReadable>,
    /// fee base which will be always added to the transaction
    #[structopt(short = "b", long)]
    fee_base: Option<u64>,
    /// fee which will be added to the transaction for every input and output
    #[structopt(short = "a", long)]
    fee_per_addr: Option<u64>,
    /// file with transaction spending keys.
    /// Must be bech32-encoded ed25519extended_secret. Required one for every input.
    #[structopt(name = "spending_key", short, long)]
    spending_keys: Vec<PathBuf>,
}

impl TxData {
    pub fn build_message(&self) -> Vec<u8> {
        if self.inputs.len() != self.spending_keys.len() {
            panic!(
                "Invalid number of spending keys ({}) should be same as inputs ({})",
                self.spending_keys.len(),
                self.inputs.len()
            )
        }
        let transaction = self.build_tx();
        let witnesses = self.spending_keys.iter().map(create_witness).collect();
        let auth_tx = AuthenticatedTransaction {
            transaction,
            witnesses,
        };
        Message::Transaction(auth_tx).serialize_as_vec().unwrap()
    }

    fn build_tx(&self) -> Transaction<Address, NoExtra> {
        let mut builder = TransactionBuilder::new();
        for input in &self.inputs {
            input.apply(&mut builder);
        }
        for output in &self.outputs {
            output.apply(&mut builder);
        }
        let fee = LinearFee::new(
            self.fee_base.unwrap_or(0),
            self.fee_per_addr.unwrap_or(0),
            0,
        );
        let output_policy = match &self.change {
            Some(addr) => OutputPolicy::One(addr.to_address()),
            None => OutputPolicy::Forget,
        };
        builder.finalize(fee, output_policy).unwrap().1
    }

    pub fn merge_old(&mut self, mut old: Self) {
        old.inputs.append(&mut self.inputs);
        old.outputs.append(&mut self.outputs);
        old.change = self.change.take().or(old.change);
        old.fee_base = self.fee_base.take().or(old.fee_base);
        old.fee_per_addr = self.fee_per_addr.take().or(old.fee_per_addr);
        old.spending_keys.append(&mut self.spending_keys);
        *self = old;
    }
}

fn create_witness(key_path: &PathBuf) -> Witness {
    let key_str = fs::read_to_string(key_path).unwrap();
    let signature = Signature::try_from_bech32_str(key_str.trim()).unwrap();
    Witness::Utxo(signature)
}
