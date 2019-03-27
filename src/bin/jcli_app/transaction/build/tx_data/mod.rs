use self::tx_address_readable::TxAddressReadable;
use self::tx_input::TxInput;
use self::tx_output::TxOutput;
use chain_core::property::Serialize as _;
use chain_crypto::bech32::Bech32;
use chain_impl_mockchain::fee::LinearFee;
use chain_impl_mockchain::key::SpendingSecretKey;
use chain_impl_mockchain::txbuilder::{
    GeneratedTransaction, OutputPolicy, TransactionBuilder, TransactionFinalizer,
};
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
    pub fn build_tx(&self) -> Vec<u8> {
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
        let (_, transaction) = builder.finalize(fee, output_policy).unwrap();
        let mut finalizer = TransactionFinalizer::new_trans(transaction);
        for spending_key in &self.spending_keys {
            apply_signature(spending_key, &mut finalizer);
        }
        let mut tx = vec![];
        match finalizer.build() {
            GeneratedTransaction::Type1(transaction) => transaction.serialize(&mut tx),
            GeneratedTransaction::Type2(transaction) => transaction.serialize(&mut tx),
        }
        .unwrap();
        tx
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

fn apply_signature(key_path: &PathBuf, finalizer: &mut TransactionFinalizer) {
    let key_str = fs::read_to_string(key_path).unwrap();
    let key = SpendingSecretKey::try_from_bech32_str(key_str.trim()).unwrap();
    finalizer.sign(&key);
}
