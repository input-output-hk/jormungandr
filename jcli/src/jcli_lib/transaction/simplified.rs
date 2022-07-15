use crate::{
    jcli_lib::{
        rest::RestArgs,
        transaction::{common, Error},
    },
    rest,
    rest::v0::message::post_fragment,
    transaction,
    transaction::{common::CommonFees, mk_witness::WitnessType, staging::Staging},
    utils::{io::ask_yes_or_no, key_parser::read_secret_key, AccountId},
};
use chain_addr::Kind;
use chain_core::property::FromStr;
use chain_crypto::{Ed25519, Ed25519Extended, PublicKey, SecretKey};
use chain_impl_mockchain::{
    account::SpendingCounter, fee::FeeAlgorithm, key::EitherEd25519SecretKey, transaction::Output,
};
use jormungandr_lib::{interfaces, interfaces::SettingsDto};
use rand::{rngs::OsRng, SeedableRng};
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

    /// the account to send funds to
    #[structopt(long)]
    pub receiver: Option<interfaces::Address>,

    #[structopt(long)]
    pub block0_hash: String,

    #[structopt(long)]
    pub valid_until: interfaces::BlockDate,

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

    // force transaction without requesting for confirmation
    #[structopt(long)]
    force: bool,

    #[structopt(long)]
    post: bool,
}

impl MakeTransaction {
    pub fn exec(self) -> Result<(), Error> {
        let secret_key = read_secret_key(self.secret)?;
        let receiver_address = if let Some(address) = self.receiver {
            address
        } else {
            let (_, address) = create_receiver_secret_key_and_address()?;
            address
        };
        let transaction = make_transaction(
            self.sender_account,
            receiver_address,
            secret_key,
            self.value,
            &self.block0_hash,
            self.valid_until,
            self.rest_args.clone(),
            self.change,
            self.force,
        )?;

        if self.post {
            let fragment = transaction.fragment()?;
            let fragment_id = post_fragment(self.rest_args, fragment)?;
            println!("Posted fragment id: {}", fragment_id);
        } else {
            // if not posted make the transaction available as a file
            transaction.store(&self.common.staging_file)?;
        }

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
    let fees = settings.fees.clone();
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
    valid_until: interfaces::BlockDate,
    rest_args: RestArgs,
    change: Option<interfaces::Address>,
    force: bool,
) -> Result<Staging, Error> {
    let mut transaction = Staging::new();

    let settings = rest::v0::settings::request_settings(rest_args.clone())?;
    let fee = common_fee_from_settings(&settings);

    let fees = settings.fees.calculate(None, 1, 1);
    let transfer_value = value.saturating_add(fees.into());

    // ask for user confirmation after adding fees
    if !force {
        println!(
            "Total value to transfer (including fees): {}",
            transfer_value
        );
        if !ask_yes_or_no(true)? {
            return Err(Error::CancelByUser);
        }
    }

    // add account
    transaction.add_account(sender_account.clone(), transfer_value)?;

    // add output
    transaction.add_output(Output {
        address: receiver_address.into(),
        value: value.into(),
    })?;

    transaction.set_expiry_date(valid_until)?;

    // finalize
    transaction::finalize::finalize(fee, change, &mut transaction)?;

    // get transaction and block0 ids
    let block0_hash = chain_impl_mockchain::chaintypes::HeaderId::from_str(block0_hash)
        .map_err(|_| Error::InvalidBlock0HeaderHash)?;
    let transaction_sign_data_hash = transaction.transaction_sign_data_hash()?;

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
        Some(SpendingCounter::from(account_state.counters()[0])),
        &secret_key,
    )?;

    // add witness
    transaction.add_witness(witness)?;

    // seal
    transaction.seal()?;

    Ok(transaction)
}
