use std::path::PathBuf;
use structopt::StructOpt;

mod get_stake_pool_id;
mod new_stake_pool_registration;
mod sign;

custom_error! {pub Error
    CannotCreatePoolRegistration { source: new_stake_pool_registration::Error } = "Cannot create new stake pool registration certificate",
    CannotSignCertificate { source: sign::Error } = "Cannot sign certificate",
    CannotGetStakePoolId { source: get_stake_pool_id::Error } = "Cannot get stake pool id from the certificate",
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
}

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum NewArgs {
    /// build a stake poole registration certificate
    StakePoolRegistration(new_stake_pool_registration::StakePoolRegistration),
}

#[derive(StructOpt)]
pub struct StakeKeyRegistrationArgs {
    /// stake pool signing public key
    #[structopt(name = "PUBLIC_KEY")]
    pub key: String,
    /// stake pool signing public key
    #[structopt(name = "SIGNING_KEY")]
    pub private_key: PathBuf,
}

#[derive(StructOpt)]
pub struct StakeKeyDeregistrationArgs {
    #[structopt(name = "PUBLIC_KEY")]
    pub key: String,
    #[structopt(name = "SIGNING_KEY")]
    pub private_key: PathBuf,
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
        }
        Ok(())
    }
}

impl Certificate {
    pub fn exec(self) -> Result<(), Error> {
        match self {
            Certificate::New(args) => args.exec()?,
            Certificate::Sign(args) => args.exec()?,
            Certificate::GetStakePoolId(args) => args.exec()?,
        }

        Ok(())
    }
}
