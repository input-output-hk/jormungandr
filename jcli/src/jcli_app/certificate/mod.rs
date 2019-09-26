use jcli_app::utils::{io, key_parser};
use jormungandr_lib::interfaces::{Certificate as CertificateType, CertificateFromStrError};
use std::fmt::Display;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use structopt::StructOpt;

mod get_stake_pool_id;
mod new_stake_delegation;
mod new_stake_pool_registration;
mod sign;

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
    /// build a stake pool registration certificate
    StakePoolRegistration(new_stake_pool_registration::StakePoolRegistration),
    /// build a stake delegation certificate
    StakeDelegation(new_stake_delegation::StakeDelegation),
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
        }
        Ok(())
    }
}

impl PrintArgs {
    pub fn exec(self) -> Result<(), Error> {
        let cert = read_cert(self.input)?;
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

fn read_cert(input: Option<PathBuf>) -> Result<CertificateType, Error> {
    use std::str::FromStr as _;

    let cert_str = read_input(input)?;
    let cert = CertificateType::from_str(&cert_str.trim_end())?;
    Ok(cert)
}

fn read_input(input: Option<PathBuf>) -> Result<String, Error> {
    let reader = io::open_file_read(&input).map_err(|source| Error::InputInvalid {
        source,
        path: input.unwrap_or_default(),
    })?;
    let mut input_str = String::new();
    BufReader::new(reader).read_line(&mut input_str)?;
    Ok(input_str)
}

fn write_cert(output: Option<PathBuf>, cert: CertificateType) -> Result<(), Error> {
    write_output(output, cert)
}

fn write_output(output: Option<PathBuf>, data: impl Display) -> Result<(), Error> {
    let mut writer = io::open_file_write(&output).map_err(|source| Error::OutputInvalid {
        source,
        path: output.unwrap_or_default(),
    })?;
    writeln!(writer, "{}", data)?;
    Ok(())
}
