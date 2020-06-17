use crate::jcli_app::utils::{io, key_parser};
use chain_impl_mockchain::block::BlockDate;
use jormungandr_lib::interfaces::{self, CertificateFromBech32Error, CertificateFromStrError};
use std::{
    fmt::Display,
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
};
use structopt::StructOpt;
use thiserror::Error;

mod get_stake_pool_id;
mod get_vote_plan_id;
mod new_owner_stake_delegation;
mod new_stake_delegation;
mod new_stake_pool_registration;
mod new_stake_pool_retirement;
mod new_vote_plan;
mod new_vote_tally;
mod sign;
mod weighted_pool_ids;

pub(crate) use self::sign::{
    committee_vote_plan_sign, committee_vote_tally_sign, pool_owner_sign,
    stake_delegation_account_binding_sign,
};

#[derive(Debug, Error)]
pub enum Error {
    #[error("invalid private key")]
    KeyInvalid(#[from] key_parser::Error),
    #[error("I/O Error")]
    Io(#[from] std::io::Error),
    #[error("invalid certificate, expecting a stake pool registration")]
    NotStakePoolRegistration,
    #[error("invalid input file path '{path}'")]
    InputInvalid {
        #[source]
        source: std::io::Error,
        path: PathBuf,
    },
    #[error("invalid output file path '{path}'")]
    OutputInvalid {
        #[source]
        source: std::io::Error,
        path: PathBuf,
    },
    #[error("Invalid certificate")]
    InvalidCertificate(#[from] CertificateFromStrError),
    #[error("Invalid certificate bech32")]
    InvalidCertificateBech32(#[from] CertificateFromBech32Error),
    #[error("invalid management_threshold value, expected between at least 1 and {max_expected} but got {got}")]
    ManagementThresholdInvalid { got: usize, max_expected: usize },
    #[error("No signing keys specified (use -k or --key to specify)")]
    NoSigningKeys,
    #[error("expecting only one signing keys but got {got}")]
    ExpectingOnlyOneSigningKey { got: usize },
    #[error("owner stake delegation does not need a signature")]
    OwnerStakeDelegationDoesntNeedSignature,
    #[error("vote plan certificate does not need a signature")]
    VotePlanDoesntNeedSignature,
    #[error("vote cast certificate does not need a signature")]
    VoteCastDoesntNeedSignature,
    #[error("secret key number {index} matching the expected public key has not been found")]
    KeyNotFound { index: usize },
    #[error("Invalid input, expected Signed Certificate or just Certificate")]
    ExpectedSignedOrNotCertificate,
    #[error("Invalid data")]
    InvalidBech32(#[from] bech32::Error),
    #[error("attempted to build delegation with zero weight")]
    PoolDelegationWithZeroWeight,
    #[error("pool delegation rates sum up to {actual}, maximum is 255")]
    InvalidPoolDelegationWeights { actual: u64, max: u64 },
    #[error("attempted to build delegation to {actual} pools, maximum is {max}")]
    TooManyPoolDelegations { actual: usize, max: usize },
    #[error("failed to build pool delegation")]
    InvalidPoolDelegation,
    #[error("BlockDates should be consecutive, vote start ({vote_start}) cannot be bigger than vote end ({vote_end})")]
    InvalidVotePlanVoteBlockDates {
        vote_start: BlockDate,
        vote_end: BlockDate,
    },
    #[error("BlockDates should be consecutive, vote end ({vote_end}) cannot be bigger committee end ({committee_end})")]
    InvalidVotePlanCommitteeBlockDates {
        vote_end: BlockDate,
        committee_end: BlockDate,
    },
    #[error("attempted to build vote plan with {actual} proposals, maximum is {max}")]
    TooManyVotePlanProposals { actual: usize, max: usize },
    #[error("invalid certificate, expecting a vote plan one")]
    NotVotePlanCertificate,
}

#[allow(clippy::large_enum_variant)]
#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum Certificate {
    /// Build certificate
    New(NewArgs),
    /// Sign certificate, you can call this command multiple
    /// time to add multiple signatures if this is required.
    Sign(sign::Sign),
    /// get the stake pool id from the given stake pool registration certificate
    GetStakePoolId(get_stake_pool_id::GetStakePoolId),
    /// get the vote plan id from the given vote plan certificate
    GetVotePlanId(get_vote_plan_id::GetVotePlanId),
    /// Print certificate
    Print(PrintArgs),
}

#[allow(clippy::large_enum_variant)]
#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum NewArgs {
    /// create the stake pool registration certificate.
    ///
    /// This contains all the declaration data of a stake pool. Including the management
    /// data. Once registered and accepted the users can delegate stake to the stake pool
    /// by referring the stake pool id.
    ///
    /// `--tax-*` parameters allow to set the rewards the stake pool will take before
    /// serving the stake delegators. If the total reward for a stake pool is `Y`. The
    /// stake pool will take a fixed (`--tax-fixed`) first: `X`. Then will take a percentage
    /// of the remaining rewards (`--tax-ratio`): `R`. The total of the rewards gained from `R`
    /// can be capped by an optional `--tax-limit`: `L` where the actual tax `T` is `X` plus
    /// the minimum of `L` and `R`.
    ///
    /// Delegators will then receive a share of the remaining rewards: `Y - T`.
    ///
    StakePoolRegistration(new_stake_pool_registration::StakePoolRegistration),
    /// build a stake delegation certificate
    StakeDelegation(new_stake_delegation::StakeDelegation),
    /// build an owner stake delegation certificate
    OwnerStakeDelegation(new_owner_stake_delegation::OwnerStakeDelegation),
    /// retire the given stake pool ID From the blockchain
    ///
    /// by doing so all remaining stake delegated to this stake pool will
    /// become pending and will need to be re-delegated.
    StakePoolRetirement(new_stake_pool_retirement::StakePoolRetirement),
    /// create a new vote plan certificate
    VotePlan(new_vote_plan::VotePlanRegistration),
    /// create a new vote tally certificate
    VoteTally(new_vote_tally::VoteTallyRegistration),
}

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct PrintArgs {
    /// get the certificate to sign from the given file. If no file
    /// provided, it will be read from the standard input
    pub input: Option<PathBuf>,
}

#[derive(StructOpt)]
pub struct StakeDelegationArgs {
    #[structopt(name = "PUBLIC_KEY")]
    pub key: String,
    #[structopt(name = "POOL_ID")]
    pub pool_id: String,
    #[structopt(name = "SIGNING_KEY")]
    pub private_key: PathBuf,
}

impl NewArgs {
    pub fn exec(self) -> Result<(), Error> {
        match self {
            NewArgs::StakePoolRegistration(args) => args.exec()?,
            NewArgs::StakeDelegation(args) => args.exec()?,
            NewArgs::OwnerStakeDelegation(args) => args.exec()?,
            NewArgs::StakePoolRetirement(args) => args.exec()?,
            NewArgs::VotePlan(args) => args.exec()?,
            NewArgs::VoteTally(args) => args.exec()?,
        }
        Ok(())
    }
}

impl PrintArgs {
    pub fn exec(self) -> Result<(), Error> {
        let cert = read_cert_or_signed_cert(self.input.as_deref())?;
        println!("{:?}", cert);
        Ok(())
    }
}

impl Certificate {
    pub fn exec(self) -> Result<(), Error> {
        match self {
            Certificate::New(args) => args.exec()?,
            Certificate::Sign(args) => args.exec()?,
            Certificate::Print(args) => args.exec()?,
            Certificate::GetStakePoolId(args) => args.exec()?,
            Certificate::GetVotePlanId(args) => args.exec()?,
        }

        Ok(())
    }
}

fn read_cert_or_signed_cert(input: Option<&Path>) -> Result<interfaces::Certificate, Error> {
    let cert_str = read_input(input)?.trim_end().to_owned();
    let (hrp, _) = bech32::decode(&cert_str)?;

    match hrp.as_ref() {
        interfaces::SIGNED_CERTIFICATE_HRP => {
            use chain_impl_mockchain::certificate::{Certificate, SignedCertificate};
            let signed_cert = interfaces::SignedCertificate::from_bech32(&cert_str)?;

            let cert = match signed_cert.0 {
                SignedCertificate::StakeDelegation(sd, _) => Certificate::StakeDelegation(sd),
                SignedCertificate::OwnerStakeDelegation(osd, _) => {
                    Certificate::OwnerStakeDelegation(osd)
                }
                SignedCertificate::PoolRegistration(pr, _) => Certificate::PoolRegistration(pr),
                SignedCertificate::PoolRetirement(pr, _) => Certificate::PoolRetirement(pr),
                SignedCertificate::PoolUpdate(pu, _) => Certificate::PoolUpdate(pu),
                SignedCertificate::VotePlan(vp, _) => Certificate::VotePlan(vp),
                SignedCertificate::VoteTally(vt, _) => Certificate::VoteTally(vt),
            };

            Ok(interfaces::Certificate(cert))
        }
        interfaces::CERTIFICATE_HRP => {
            interfaces::Certificate::from_bech32(&cert_str).map_err(Error::from)
        }
        _ => Err(Error::ExpectedSignedOrNotCertificate),
    }
}

fn read_cert(input: Option<&Path>) -> Result<interfaces::Certificate, Error> {
    use std::str::FromStr as _;

    let cert_str = read_input(input)?;
    let cert = interfaces::Certificate::from_str(&cert_str.trim_end())?;
    Ok(cert)
}

pub(crate) fn read_input(input: Option<&Path>) -> Result<String, Error> {
    let reader = io::open_file_read(&input).map_err(|source| Error::InputInvalid {
        source,
        path: input.map(|x| x.to_path_buf()).unwrap_or_default(),
    })?;
    let mut input_str = String::new();
    BufReader::new(reader).read_line(&mut input_str)?;
    Ok(input_str)
}

fn write_cert<P>(output: Option<P>, cert: interfaces::Certificate) -> Result<(), Error>
where
    P: AsRef<Path>,
{
    write_output(output, cert)
}

fn write_signed_cert(
    output: Option<&Path>,
    signedcert: interfaces::SignedCertificate,
) -> Result<(), Error> {
    write_output(output, signedcert)
}

fn write_output<P>(output: Option<P>, data: impl Display) -> Result<(), Error>
where
    P: AsRef<Path>,
{
    let mut writer = io::open_file_write(&output).map_err(|source| Error::OutputInvalid {
        source,
        path: output.map(|x| x.as_ref().to_path_buf()).unwrap_or_default(),
    })?;
    writeln!(writer, "{}", data)?;
    Ok(())
}
