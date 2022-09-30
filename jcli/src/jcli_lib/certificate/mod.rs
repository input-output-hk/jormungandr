#[cfg(feature = "evm")]
mod new_evm_mapping;
mod new_owner_stake_delegation;
mod new_stake_delegation;
mod new_stake_pool_registration;
mod new_stake_pool_retirement;
mod new_update_proposal;
mod new_update_vote;
mod new_vote_cast;
mod new_vote_plan;
mod new_vote_tally;
mod show;
mod sign;
mod weighted_pool_ids;

pub(crate) use self::sign::{
    committee_vote_plan_sign, committee_vote_tally_sign, evm_mapping_sign, pool_owner_sign,
    stake_delegation_account_binding_sign, update_proposal_sign, update_vote_sign,
};
use crate::jcli_lib::utils::{
    io, key_parser,
    vote::{SharesError, VotePlanError},
};
use chain_impl_mockchain::{block::BlockDate, certificate::DecryptedPrivateTallyError};
use jormungandr_lib::interfaces::{self, CertificateFromBech32Error, CertificateFromStrError};
use std::{
    fmt::Display,
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
};
use structopt::StructOpt;
use thiserror::Error;

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
    #[error("mint token does not need a signature")]
    MintTokenDoesntNeedSignature,
    #[error("vote plan certificate does not need a signature")]
    VotePlanDoesntNeedSignature,
    #[error("vote cast certificate does not need a signature")]
    VoteCastDoesntNeedSignature,
    #[error("secret key number {index} matching the expected public key has not been found")]
    KeyNotFound { index: usize },
    #[error("Invalid input, expected Signed Certificate or just Certificate")]
    ExpectedSignedOrNotCertificate,
    #[error("Invalid bech32 data")]
    InvalidBech32(#[from] chain_crypto::bech32::Error),
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
    #[error("invalid vote plan certificate configuration")]
    VotePlanConfig(#[source] serde_yaml::Error),
    #[error("invalid base64 encoded bytes")]
    Base64(#[source] base64::DecodeError),
    #[error("invalid election public key")]
    ElectionPublicKey,
    #[error("invalid bech32 public key, expected {expected} hrp got {actual}")]
    InvalidBech32Key { expected: String, actual: String },
    #[error("invalid shares JSON representation")]
    InvalidJson(#[from] serde_json::Error),
    #[error("private vote plans `committee_public_keys` cannot be empty")]
    InvalidPrivateVotePlanCommitteeKeys,
    #[error(transparent)]
    VotePlanError(#[from] VotePlanError),
    #[error(transparent)]
    SharesError(#[from] SharesError),
    #[error("expected decrypted private tally, found {found}")]
    PrivateTallyExpected { found: &'static str },
    #[error(transparent)]
    PrivateTallyError(#[from] DecryptedPrivateTallyError),
    #[error("config file corrupted")]
    ConfigFileCorrupted(#[source] serde_yaml::Error),
}

#[allow(clippy::large_enum_variant)]
#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum Certificate {
    /// Build certificate
    New(NewArgs),
    /// Sign a certificate. You can call this command multiple
    /// times to add multiple signatures if this is required.
    Sign(sign::Sign),
    /// Output information encoded into the certificate
    Show(show::ShowArgs),
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
    /// create a new update vote certificate
    UpdateVote(new_update_vote::UpdateVote),
    /// create a new update proposal certificate
    UpdateProposal(new_update_proposal::UpdateProposal),
    /// create a vote cast certificate
    VoteCast(new_vote_cast::VoteCastCmd),
    #[cfg(feature = "evm")]
    /// create an EVM address mapping certificate
    EvmMapping(new_evm_mapping::EvmMapCmd),
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
            NewArgs::VoteCast(args) => args.exec()?,
            NewArgs::UpdateVote(args) => args.exec()?,
            NewArgs::UpdateProposal(args) => args.exec()?,
            #[cfg(feature = "evm")]
            NewArgs::EvmMapping(args) => args.exec()?,
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
            Certificate::Show(args) => args.exec()?,
        }

        Ok(())
    }
}

fn read_cert_or_signed_cert(input: Option<&Path>) -> Result<interfaces::Certificate, Error> {
    let cert_str = read_input(input)?.trim_end().to_owned();
    let (hrp, _, _variant) =
        bech32::decode(&cert_str).map_err(chain_crypto::bech32::Error::from)?;

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
                SignedCertificate::UpdateProposal(vt, _) => Certificate::UpdateProposal(vt),
                SignedCertificate::UpdateVote(vt, _) => Certificate::UpdateVote(vt),
                SignedCertificate::EvmMapping(vt, _) => Certificate::EvmMapping(vt),
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
    let cert = interfaces::Certificate::from_str(cert_str.trim_end())?;
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
