use super::io;
use chain_crypto::{
    bech32::{self, Bech32},
    AsymmetricKey, AsymmetricPublicKey, PublicKey, SecretKey,
};
use chain_impl_mockchain::key::EitherEd25519SecretKey;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("could not open secret key file '{path}': {source}")]
    SecretKeyFileReadFailed {
        #[source]
        source: std::io::Error,
        path: PathBuf,
    },
    #[error("could not decode secret file '{path}': {source}")]
    SecretKeyFileMalformed {
        #[source]
        source: bech32::Error,
        path: PathBuf,
    },
    #[error("could not decode secretkey: {0}")]
    SecretKeyMalformed(#[from] bech32::Error),
    #[error("error requesting user input")]
    UserInputError(#[from] std::io::Error),
}

pub fn parse_pub_key<A: AsymmetricPublicKey>(
    bech32_str: &str,
) -> Result<PublicKey<A>, bech32::Error> {
    Bech32::try_from_bech32_str(bech32_str)
}

pub fn _read_secret_key_from_file<A, P>(path: &Option<P>) -> Result<SecretKey<A>, Error>
where
    A: AsymmetricKey,
    SecretKey<A>: Bech32,
    P: AsRef<Path>,
{
    let bech32_str: String =
        io::read_line(path).map_err(|source| Error::SecretKeyFileReadFailed {
            source,
            path: io::path_to_path_buf(path),
        })?;
    SecretKey::try_from_bech32_str(&bech32_str).map_err(|source| Error::SecretKeyFileMalformed {
        source,
        path: io::path_to_path_buf(path),
    })
}

pub fn read_secret_key(secret_key_path: Option<PathBuf>) -> Result<EitherEd25519SecretKey, Error> {
    match secret_key_path {
        Some(path) => read_ed25519_secret_key_from_file(&Some(path)),
        None => {
            let key = rpassword::prompt_password("Introduce the bech32 format secret key:\n")?;
            parse_ed25519_secret_key(&key)
        }
    }
}

pub fn read_ed25519_secret_key_from_file<P: AsRef<Path>>(
    path: &Option<P>,
) -> Result<EitherEd25519SecretKey, Error> {
    let bech32_str: String =
        io::read_line(path).map_err(|source| Error::SecretKeyFileReadFailed {
            source,
            path: io::path_to_path_buf(path),
        })?;

    match SecretKey::try_from_bech32_str(&bech32_str) {
        Ok(sk) => Ok(EitherEd25519SecretKey::Extended(sk)),
        Err(_) => SecretKey::try_from_bech32_str(&bech32_str)
            .map(EitherEd25519SecretKey::Normal)
            .map_err(|source| Error::SecretKeyFileMalformed {
                source,
                path: io::path_to_path_buf(path),
            }),
    }
}

pub fn parse_ed25519_secret_key(bech32_str: &str) -> Result<EitherEd25519SecretKey, Error> {
    match SecretKey::try_from_bech32_str(bech32_str) {
        Ok(sk) => Ok(EitherEd25519SecretKey::Extended(sk)),
        Err(_) => SecretKey::try_from_bech32_str(bech32_str)
            .map(EitherEd25519SecretKey::Normal)
            .map_err(Error::SecretKeyMalformed),
    }
}
