use crate::jcli_lib::rest::RestArgs;
use crate::jcli_lib::transaction::{common, Error};
use crate::transaction::mk_witness::WitnessType;
use crate::transaction::staging::Staging;
use crate::utils::key_parser::read_ed25519_secret_key_from_file;
use crate::utils::AccountId;
use crate::{rest, transaction};
use chain_addr::Kind;
use chain_core::property::FromStr;
use chain_crypto::{Ed25519, Ed25519Extended, PublicKey, SecretKey};
use chain_impl_mockchain::account::SpendingCounter;
use chain_impl_mockchain::key::EitherEd25519SecretKey;
use chain_impl_mockchain::transaction::Output;
use jormungandr_lib::interfaces;

use crate::transaction::common::CommonFees;
use jormungandr_lib::interfaces::SettingsDto;
use rand::rngs::OsRng;
use rand::SeedableRng;
use rand_chacha::ChaChaRng;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct MakeTransaction {
    /// the account to debit the funds from
    #[structopt(name = "ACCOUNT")]
    pub sender_account: interfaces::Address,

    /// the value
    #[structopt(name = "VALUE")]
    pub value: interfaces::Value,

    #[structopt(long)]
    pub block0_hash: String,

    /// the file path to the file to read the signing key from.
    /// If omitted it will be read from the standard input.
    #[structopt(long)]
    pub secret: Option<PathBuf>,

    /// Set the change in the given address
    #[structopt(long)]
    pub change: Option<interfaces::Address>,

    #[structopt(flatten)]
    pub common: common::CommonTransaction,

    #[structopt(flatten)]
    rest_args: RestArgs,
}

impl MakeTransaction {
    pub fn exec(self) -> Result<(), Error> {
        let secret_key = read_ed25519_secret_key_from_file(&self.secret)?;
        let (_receiver_secret_key, receiver_address) = create_receiver_secret_key_and_address()?;
        let transaction = make_transaction(
            self.sender_account,
            receiver_address,
            secret_key,
            self.value,
            &self.block0_hash,
            self.rest_args.clone(),
            self.change,
        )?;
        transaction.store(&self.common.staging_file)?;
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

fn common_fee_from_settings(settings: &SettingsDto) -> CommonFees {
    let fees = settings.fees;
    CommonFees {
        constant: fees.constant,
        coefficient: fees.coefficient,
        certificate: fees.certificate,
        certificate_pool_registration: fees
            .per_certificate_fees
            .certificate_pool_registration
            .map(Into::into),
        certificate_stake_delegation: fees
            .per_certificate_fees
            .certificate_owner_stake_delegation
            .map(Into::into),
        certificate_owner_stake_delegation: fees
            .per_certificate_fees
            .certificate_owner_stake_delegation
            .map(Into::into),
        certificate_vote_plan: fees
            .per_vote_certificate_fees
            .certificate_vote_plan
            .map(Into::into),
        certificate_vote_cast: fees
            .per_vote_certificate_fees
            .certificate_vote_cast
            .map(Into::into),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn make_transaction(
    sender_account: interfaces::Address,
    receiver_address: interfaces::Address,
    secret_key: EitherEd25519SecretKey,
    value: interfaces::Value,
    block0_hash: &str,
    rest_args: RestArgs,
    change: Option<interfaces::Address>,
) -> Result<Staging, Error> {
    let mut transaction = Staging::new();

    // add account
    transaction.add_account(sender_account.clone(), value)?;

    // add output
    transaction.add_output(Output {
        address: receiver_address.into(),
        value: value.into(),
    })?;

    let settings = rest::v0::settings::request_settings(rest_args.clone())?;
    let fee = common_fee_from_settings(&settings);

    // finalize
    transaction::finalize::finalize(fee, change, &mut transaction)?;

    // get transaction and block0 ids
    let block0_hash = chain_impl_mockchain::chaintypes::HeaderId::from_str(block0_hash)
        .map_err(|_| Error::InvalidBlock0HeaderHash)?;
    let transaction_sign_data_hash = transaction.transaction_sign_data_hash();

    // get spending counter
    let account_state = rest::v0::account::request_account_information(
        rest_args,
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

    Ok(transaction)
}
