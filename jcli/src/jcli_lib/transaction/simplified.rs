use crate::block::{load_block, open_block_file};
use crate::jcli_lib::rest::RestArgs;
use crate::jcli_lib::transaction::{common, Error};
use chain_crypto::{bech32::Bech32 as _, AsymmetricKey, Ed25519Extended, SecretKey};

use crate::transaction::mk_witness::WitnessType;
use crate::transaction::staging::Staging;
use crate::utils::key_parser::read_ed25519_secret_key_from_file;
use crate::utils::AccountId;
use crate::{address, rest, transaction};
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
pub struct SimplifiedTransaction {
    /// the file path to the file to read the signing key from.
    /// If omitted it will be read from the standard input.
    pub secret: Option<PathBuf>,

    /// the account to debit the funds from
    #[structopt(name = "ACCOUNT")]
    pub faucet_address: interfaces::Address,

    /// the value
    #[structopt(name = "VALUE")]
    pub value: interfaces::Value,

    #[structopt(flatten)]
    pub common: common::CommonTransaction,

    #[structopt(flatten)]
    pub fee: common::CommonFees,

    /// Set the change in the given address
    pub change: Option<interfaces::Address>,

    #[structopt(default_value = "block-0.bin")]
    pub block0_path: PathBuf,

    #[structopt(flatten)]
    rest_args: RestArgs,
}

impl SimplifiedTransaction {
    pub fn exec(self) -> Result<(), Error> {
        let secret_key = read_ed25519_secret_key_from_file(&self.secret)?;
        let (receiver_secret_key, receiver_address) = create_receiver_secret_key_and_address()?;
        let fragment_id = simplified_transaction(
            self.faucet_address,
            receiver_address,
            secret_key,
            self.value,
            self.fee,
            self.block0_path,
            self.rest_args.clone(),
            self.change,
        )?;
        println!("{}", fragment_id);
        println!(
            "Private key of receiver (to revert transaction for testing purposes): {}",
            receiver_secret_key.to_bech32_str()
        );
        println!("To see if transaction is in block use:");
        println!("jcli rest v0 message logs -h {}", &self.rest_args.host);
        println!("To check new account balance :");
        println!(
            "jcli  rest v0 account get  $RECEIVER_ADDR -h {}",
            &self.rest_args.host
        );
        Ok(())
    }
}

fn create_new_private_key() -> Result<SecretKey<Ed25519Extended>, Error> {
    let rng = ChaChaRng::from_rng(OsRng)?;
    let key = SecretKey::<Ed25519Extended>::generate(rng);
    Ok(key)
}

fn create_receiver_address(sk: &EitherEd25519SecretKey) -> Result<interfaces::Address, Error> {
    let sk = create_new_private_key()?;
    let pk = sk.to_public();
    let address = interfaces::Address::;
    let address = address::mk_account(Ed25519Extended::SECRET_BECH32_HRP, pk, true)?;
    Ok(address)
}

fn create_receiver_secret_key_and_address(
) -> Result<(SecretKey<Ed25519Extended>, interfaces::Address), Error> {
    let pk = create_new_private_key()?;
    let key = EitherEd25519SecretKey::Extended(pk.clone());
    let address = create_receiver_address(&key)?;
    Ok((pk, address))
}

#[allow(clippy::too_many_arguments)]
pub fn simplified_transaction(
    faucet_address: interfaces::Address,
    receiver_address: interfaces::Address,
    secret_key: EitherEd25519SecretKey,
    value: interfaces::Value,
    fee: common::CommonFees,
    block0_file: PathBuf,
    rest_args: RestArgs,
    change: Option<interfaces::Address>,
) -> Result<String, Error> {
    let mut transaction = Staging::new();

    // add account
    transaction::add_account::add_account(faucet_address.clone(), value, &mut transaction)?;

    // add output
    transaction.add_output(Output {
        address: receiver_address.into(),
        value: value.into(),
    })?;

    //finalize
    transaction::finalize::finalize(fee, change, &mut transaction)?;

    // get transaction and block0 ids
    let transaction_sign_data_hash = transaction.transaction_sign_data_hash();
    let block0 = load_block(open_block_file(&Some(block0_file))?)?;
    let block0_hash = block0.header.id();

    // get spending counter
    let account_state = rest::v0::account::request_account_information(
        rest_args.clone(),
        AccountId::try_from_str(&faucet_address.to_string())?,
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
