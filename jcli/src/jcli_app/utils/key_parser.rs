use super::io;
use chain_crypto::bech32::{self, Bech32};
use chain_crypto::{AsymmetricKey, AsymmetricPublicKey, PublicKey, SecretKey};
use chain_impl_mockchain::key::EitherEd25519SecretKey;
use std::path::{Path, PathBuf};

custom_error! { pub Error
    SecretKeyFileReadFailed { source: std::io::Error, path: PathBuf }
        = @{{ let _ = source; format_args!("could not open secret key file '{}': {}", path.display(), source) }},
    SecretKeyFileMalformed { source: bech32::Error, path: PathBuf }
        = @{{ format_args!("could not decode secret file '{}': {}", path.display(), source) }},
    SecretKeyMalformed { source: bech32::Error }
        = @{{ format_args!("could not decode secretkey: {}", source) }},
}

pub fn parse_pub_key<A: AsymmetricPublicKey>(
    bech32_str: &str,
) -> Result<PublicKey<A>, bech32::Error> {
    Bech32::try_from_bech32_str(bech32_str)
}

pub fn read_secret_key_from_file<A: AsymmetricKey, P: AsRef<Path>>(
    path: &Option<P>,
) -> Result<SecretKey<A>, Error> {
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
            .map(|sk| EitherEd25519SecretKey::Normal(sk))
            .map_err(|source| Error::SecretKeyFileMalformed {
                source,
                path: io::path_to_path_buf(path),
            }),
    }
}

pub fn parse_ed25519_secret_key(bech32_str: &str) -> Result<EitherEd25519SecretKey, Error> {
    match SecretKey::try_from_bech32_str(&bech32_str) {
        Ok(sk) => Ok(EitherEd25519SecretKey::Extended(sk)),
        Err(_) => SecretKey::try_from_bech32_str(&bech32_str)
            .map(|sk| EitherEd25519SecretKey::Normal(sk))
            .map_err(|source| Error::SecretKeyMalformed { source }),
    }
}
