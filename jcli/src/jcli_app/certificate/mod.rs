use crate::jcli_app::utils::{io, key_parser};
use jormungandr_lib::interfaces::{self, CertificateFromStrError};
use std::fmt::Display;
use std::io::{BufRead, BufReader, Write};
use std::ops::Deref;
use std::path::{Path, PathBuf};
use structopt::StructOpt;

mod get_stake_pool_id;
mod new_owner_stake_delegation;
mod new_stake_delegation;
mod new_stake_pool_registration;
mod sign;
mod weighted_pool_ids;

pub(crate) use self::sign::{pool_owner_sign, stake_delegation_account_binding_sign};

custom_error! {pub Error
    KeyInvalid { source: key_parser::Error } = "invalid private key",
    Io { source: std::io::Error } = "I/O Error",
    NotStakePoolRegistration = "invalid certificate, expecting a stake pool registration",
    InputInvalid { source: std::io::Error, path: PathBuf }
        = @{{ let _ = source; format_args!("invalid input file path '{}'", path.display()) }},
    OutputInvalid { source: std::io::Error, path: PathBuf }
        = @{{ let _ = source; format_args!("invalid output file path '{}'", path.display()) }},
    InvalidCertificate { source: CertificateFromStrError } = "Invalid certificate",
    ManagementThresholdInvalid { got: usize, max_expected: usize }
        = "invalid management_threshold value, expected between at least 1 and {max_expected} but got {got}",
    NoSigningKeys = "No signing keys specified (use -k or --key to specify)",
    ExpectingOnlyOneSigningKey { got: usize }
        = "expecting only one signing keys but got {got}",
    OwnerStakeDelegationDoesntNeedSignature = "owner stake delegation does not need a signature",
    KeyNotFound { index: usize }
        = "secret key number {index} matching the expected public key has not been found",
    ExpectedSignedOrNotCertificate = "Invalid input, expected Signed Certificate or just Certificate",
    InvalidBech32 { source: bech32::Error } = "Invalid data",
    PoolDelegationWithZeroWeight = "attempted to build delegation with zero weight",
    InvalidPoolDelegationWeights { actual: u64, max: u64 } = "pool delegation rates sum up to {actual}, maximum is 255",
    TooManyPoolDelegations { actual: usize, max: usize } = "attempted to build delegation to {actual} pools, maximum is {max}",
    InvalidPoolDelegation = "failed to build pool delegation",
}

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
    /// Print certificate
    Print(PrintArgs),
}

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
    /// of the remaining rewards (`--tax-ratio`): `R`. The total of the tax `X + R`
    /// can be capped by an optional `--tax-limit`: `L` where the actual tax `T` is the minimum of
    /// `L` and `X + R`.
    ///
    /// Delegators will then receive a share of the remaining rewards: `Y - T`.
    ///
    StakePoolRegistration(new_stake_pool_registration::StakePoolRegistration),
    /// build a stake delegation certificate
    StakeDelegation(new_stake_delegation::StakeDelegation),
    /// build an owner stake delegation certificate
    OwnerStakeDelegation(new_owner_stake_delegation::OwnerStakeDelegation),
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
        }
        Ok(())
    }
}

impl PrintArgs {
    pub fn exec(self) -> Result<(), Error> {
        let cert = read_cert_or_signed_cert(self.input.as_ref().map(|x| x.deref()))?;
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
        }

        Ok(())
    }
}

fn read_cert_or_signed_cert(input: Option<&Path>) -> Result<interfaces::Certificate, Error> {
    use bech32::Bech32;

    use std::str::FromStr as _;

    let cert_str = read_input(input)?.trim_end().to_owned();
    let bech32 = Bech32::from_str(&cert_str)?;

    match bech32.hrp() {
        interfaces::SIGNED_CERTIFICATE_HRP => {
            use chain_impl_mockchain::certificate::{Certificate, SignedCertificate};
            let signed_cert = interfaces::SignedCertificate::from_str(&cert_str)?;

            let cert = match signed_cert.0 {
                SignedCertificate::StakeDelegation(sd, _) => Certificate::StakeDelegation(sd),
                SignedCertificate::OwnerStakeDelegation(osd, _) => {
                    Certificate::OwnerStakeDelegation(osd)
                }
                SignedCertificate::PoolRegistration(pr, _) => Certificate::PoolRegistration(pr),
                SignedCertificate::PoolRetirement(pr, _) => Certificate::PoolRetirement(pr),
                SignedCertificate::PoolUpdate(pu, _) => Certificate::PoolUpdate(pu),
            };

            Ok(interfaces::Certificate(cert))
        }
        interfaces::CERTIFICATE_HRP => {
            interfaces::Certificate::from_str(&cert_str).map_err(Error::from)
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

fn write_cert(output: Option<&Path>, cert: interfaces::Certificate) -> Result<(), Error> {
    write_output(output, cert)
}

fn write_signed_cert(
    output: Option<&Path>,
    signedcert: interfaces::SignedCertificate,
) -> Result<(), Error> {
    write_output(output, signedcert)
}

fn write_output(output: Option<&Path>, data: impl Display) -> Result<(), Error> {
    let mut writer = io::open_file_write(&output).map_err(|source| Error::OutputInvalid {
        source,
        path: output.map(|x| x.to_path_buf()).unwrap_or_default(),
    })?;
    writeln!(writer, "{}", data)?;
    Ok(())
}
