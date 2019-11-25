use crate::jcli_app::certificate::{write_cert, Error};
use crate::jcli_app::utils::key_parser::parse_pub_key;
use chain_crypto::{Curve25519_2HashDH, Ed25519, PublicKey, SumEd25519_12};
use chain_impl_mockchain::{
    certificate::{Certificate, PoolPermissions, PoolRegistration},
    leadership::genesis::GenesisPraosLeader,
    rewards,
};
use chain_time::DurationSeconds;
use jormungandr_lib::interfaces::{Ratio, Value};
use std::ops::Deref;
use std::{
    num::{NonZeroU64, NonZeroU8},
    path::PathBuf,
};
use structopt::StructOpt;

/// create the stake pool registration certificate.
///
/// This contains all the declaration data of a stake pool. Including the management
/// data. Once registered and accepted the users can delegate stake to the stake pool
/// by referring the stake pool id.
///
/// `--tax-*` parameters allow to set the rewards the stake pool will take before
/// serving the stake delegators. If the total reward for a stake pool is `Y`. The
/// stake pool will take a fixed (`--tax-fixed`) first: `X`. Then will take a percentage
/// of the remaining rewards (`--tax-ratio`): `R`. The total of the tax `X + R`
/// can be capped by an optional `--tax-limit`: `L` where the actual tax `T` is the minimum of
/// `L` and `X + R`.
///
/// Delegators will then receive a share of the remaining rewards: `Y - T`.
///
#[derive(Debug, StructOpt)]
pub struct StakePoolRegistration {
    /// serial code for the stake pool certificate
    ///
    /// This value is arbitrary and does not need to be unique in the whole blockchain.
    /// It can be used for stake pool owners or operators to differentiate multiple
    /// stake pools they control.
    #[structopt(long = "serial", name = "SERIAL")]
    pub serial: u128,
    /// management threshold
    ///
    /// This is the number of owners keys that are required to update the stake
    /// pools parameter (the tax, update the keys, the threshold itsef...).
    #[structopt(long = "management-threshold", name = "THRESHOLD")]
    pub management_threshold: NonZeroU8,

    /// start validity
    ///
    /// This state when the stake pool registration becomes effective in seconds since
    /// the block0 start time.
    #[structopt(long = "start-validity", name = "SECONDS-SINCE-START")]
    pub start_validity: u64,

    /// public key of the owner(s)
    ///
    /// Owner can change any of the stake pool parameters as long as there
    /// is <THRESHOLD> number of owners to sign the stake pool parameters update.
    ///
    /// Owner will receive a share of the fixed and ratio tax too. unless a reward
    /// account is specified for the stake pool.
    #[structopt(
        long = "owner",
        name = "OWNER_KEY",
        parse(try_from_str = "parse_pub_key"),
        required = true
    )]
    pub owners: Vec<PublicKey<Ed25519>>,

    /// public key of the operators(s)
    ///
    /// Owners can allow an operator to update some or all of the stake pool parameters.
    /// Different operators can have different permissions.
    #[structopt(
        long = "operator",
        name = "OPERATOR_KEY",
        parse(try_from_str = "parse_pub_key")
    )]
    pub operators: Vec<PublicKey<Ed25519>>,

    /// Public key of the block signing key
    #[structopt(
        long = "kes-key",
        name = "KES_KEY",
        parse(try_from_str = "parse_pub_key")
    )]
    pub kes_key: PublicKey<SumEd25519_12>,

    /// public key of the VRF key
    #[structopt(
        long = "vrf-key",
        name = "VRF_KEY",
        parse(try_from_str = "parse_pub_key")
    )]
    pub vrf_key: PublicKey<Curve25519_2HashDH>,

    /// set the fixed value tax the stake pool will reserve from the reward
    ///
    /// For example, a stake pool may set this value to cover their fixed operation
    /// costs.
    #[structopt(long = "tax-fixed", name = "TAX_VALUE", default_value = "0")]
    pub tax_fixed: Value,

    /// The percentage take of the stake pool.
    ///
    /// Once the `tax-fixed` has been take, this is the percentage the stake pool will
    /// take for themselves.
    #[structopt(long = "tax-ratio", name = "TAX_RATIO", default_value = "0/1")]
    pub tax_ratio: Ratio,

    /// The maximum tax value the stake pool will take.
    ///
    /// This will set the maximum the stake pool value will reserve for themselves. Including
    /// both the `--tax-fixed` and the `--tax-ratio`.
    #[structopt(long = "tax-limit", name = "TAX_LIMIT")]
    pub tax_limit: Option<NonZeroU64>,

    /// print the output signed certificate in the given file, if no file given
    /// the output will be printed in the standard output
    pub output: Option<PathBuf>,
}

impl StakePoolRegistration {
    pub fn exec(self) -> Result<(), Error> {
        let rewards = rewards::TaxType {
            fixed: self.tax_fixed.into(),
            ratio: self.tax_ratio.into(),
            max_limit: self.tax_limit,
        };

        let content = PoolRegistration {
            serial: self.serial,
            owners: self.owners.clone(),
            operators: self.operators.clone().into(),
            permissions: PoolPermissions::new(self.management_threshold.get()),
            start_validity: DurationSeconds::from(self.start_validity).into(),
            rewards,
            reward_account: None,
            keys: GenesisPraosLeader {
                kes_public_key: self.kes_key,
                vrf_public_key: self.vrf_key,
            },
        };

        if self.management_threshold.get() as usize > self.owners.len() {
            return Err(Error::ManagementThresholdInvalid {
                got: self.management_threshold.get() as usize,
                max_expected: self.owners.len(),
            });
        };

        let cert = Certificate::PoolRegistration(content);
        write_cert(self.output.as_ref().map(|x| x.deref()), cert.into())
    }
}
