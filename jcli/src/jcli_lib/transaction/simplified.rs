use crate::jcli_lib::rest::RestArgs;
use crate::jcli_lib::transaction::{common, Error};
use chain_crypto::{bech32::Bech32 as _, Ed25519, Ed25519Extended, PublicKey, SecretKey};

use crate::transaction::mk_witness::WitnessType;
use crate::transaction::staging::Staging;
use crate::utils::key_parser::read_ed25519_secret_key_from_file;
use crate::utils::AccountId;
use crate::{rest, transaction};
use chain_addr::Kind;
use chain_core::property::FromStr;
use chain_impl_mockchain::account::SpendingCounter;
use chain_impl_mockchain::key::EitherEd25519SecretKey;
use chain_impl_mockchain::transaction::Output;
use jormungandr_lib::interfaces;
use rand::rngs::OsRng;
use rand::SeedableRng;
use rand_chacha::ChaChaRng;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct MakeTransaction {
    /// the file path to the file to read the signing key from.
    /// If omitted it will be read from the standard input.
    pub secret: Option<PathBuf>,

    /// the account to debit the funds from
    #[structopt(name = "ACCOUNT")]
    pub sender_account: interfaces::Address,

    /// the value
    #[structopt(name = "VALUE")]
    pub value: interfaces::Value,

    #[structopt(flatten)]
    pub common: common::CommonTransaction,

    #[structopt(flatten)]
    pub fee: common::CommonFees,

    /// Set the change in the given address
    pub change: Option<interfaces::Address>,

    pub block0_hash: String,

    #[structopt(flatten)]
    rest_args: RestArgs,
}

impl MakeTransaction {
    pub fn exec(self) -> Result<(), Error> {
        let secret_key = read_ed25519_secret_key_from_file(&self.secret)?;
        let (receiver_secret_key, receiver_address) = create_receiver_secret_key_and_address()?;
        let fragment_id = make_transaction(
            self.sender_account,
            receiver_address.clone(),
            secret_key,
            self.value,
            self.fee,
            &self.block0_hash,
            self.rest_args.clone(),
            self.change,
        )?;
        println!(
            "{}
Private key of receiver (to revert transaction for testing purposes): {}
To see if transaction is in block use:
    jcli rest v0 message logs -h {host}
To check new account balance:
    jcli  rest v0 account get  {} -h {host}
            ",
            fragment_id,
            receiver_secret_key.to_bech32_str(),
            receiver_address,
            host = &self.rest_args.host
        );
        Ok(())
    }
}

fn create_new_private_key() -> Result<SecretKey<Ed25519Extended>, Error> {
    let rng = ChaChaRng::from_rng(OsRng)?;
    let key = SecretKey::<Ed25519Extended>::generate(rng);
    Ok(key)
}

fn create_receiver_address(sk: &SecretKey<Ed25519Extended>) -> interfaces::Address {
    let pk = sk.to_public();
    make_address(pk)
}

fn make_address(pk: PublicKey<Ed25519>) -> interfaces::Address {
    chain_addr::Address(chain_addr::Discrimination::Test, Kind::Account(pk)).into()
}

fn create_receiver_secret_key_and_address(
) -> Result<(SecretKey<Ed25519Extended>, interfaces::Address), Error> {
    let sk = create_new_private_key()?;
    let address = create_receiver_address(&sk);
    Ok((sk, address))
}

#[allow(clippy::too_many_arguments)]
pub fn make_transaction(
    sender_account: interfaces::Address,
    receiver_address: interfaces::Address,
    secret_key: EitherEd25519SecretKey,
    value: interfaces::Value,
    fee: common::CommonFees,
    block0_hash: &str,
    rest_args: RestArgs,
    change: Option<interfaces::Address>,
) -> Result<String, Error> {
    let mut transaction = Staging::new();

    // add account
    transaction.add_account(sender_account.clone(), value)?;

    // add output
    transaction.add_output(Output {
        address: receiver_address.into(),
        value: value.into(),
    })?;

    // finalize
    transaction::finalize::finalize(fee, change, &mut transaction)?;

    // get transaction and block0 ids
    let block0_hash = chain_impl_mockchain::chaintypes::HeaderId::from_str(block0_hash)
        .map_err(|_| Error::InvalidBlock0HeaderHash)?;
    let transaction_sign_data_hash = transaction.transaction_sign_data_hash();

    // get spending counter
    let account_state = rest::v0::account::request_account_information(
        rest_args.clone(),
        AccountId::try_from_str(&sender_account.to_string())?,
    )?;

    // make witness
    let witness = transaction::mk_witness::make_witness(
        &WitnessType::Account,
        &block0_hash,
        &transaction_sign_data_hash,
        Some(SpendingCounter::from(account_state.counter())),
        &secret_key,
    )?;

    // add witness
    transaction.add_witness(witness)?;

    // seal
    transaction.seal()?;

    // send fragment
    let fragment = transaction.fragment()?;
    let fragment_id = rest::v0::message::post_fragment(rest_args, fragment)?;

    Ok(fragment_id)
}
