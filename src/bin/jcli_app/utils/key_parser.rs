use bech32::{Bech32, Error as Bech32Error, FromBase32};
use chain_crypto::{AsymmetricKey, PublicKey, PublicKeyError};

custom_error! {pub ParsePubKeyError
    InvalidBech32 { source: Bech32Error } = "Invalid encoding (not valid bech32): {source}",
    NotValidHrp { actual: String, expected: String } = "Invalid prefix, expected {expected} but received {actual}",
    PublicKey { source: PublicKeyError } = "Invalid public key: {source}",
}

pub fn parse_pub_key<A: AsymmetricKey>(s: &str) -> Result<PublicKey<A>, ParsePubKeyError> {
    let bech32: Bech32 = s.parse()?;
    if bech32.hrp() == A::PUBLIC_BECH32_HRP {
        let pub_key_bytes = Vec::<u8>::from_base32(bech32.data())?;
        Ok(PublicKey::from_binary(&pub_key_bytes)?)
    } else {
        Err(ParsePubKeyError::NotValidHrp {
            actual: bech32.hrp().to_owned(),
            expected: A::PUBLIC_BECH32_HRP.to_owned(),
        })
    }
}
