//! # Account Signing Key and Identifier
//!
//! While managing account there are a couple of items the user will
//! need to manage. The first one being their [`SigningKey`]. This is
//! the key that will be used to sign transactions. Owning this key
//! means that any value associated to this **Account** can be spent.
//!
//! From the [`SigningKey`] we can extract the public [`Identifier`].
//! This is the data that will be used to publicly represent our
//! **Account** on the blockchain.
//!
//! # Example
//!
//! ```
//! # use chain_addr::{Discrimination, AddressReadable};
//! # use jormungandr_lib::crypto::account::{SigningKey, Identifier};
//! # use rand::thread_rng;
//!
//! // generate a signing key
//! let signing_key = SigningKey::generate(thread_rng());
//!
//! // extract the associated identifier
//! let identifier = signing_key.identifier();
//!
//! // get the address to this account:
//! let address = identifier.to_address(Discrimination::Test);
//!
//! println!(
//!   "Please, send money to my account: {}",
//!   AddressReadable::from_address("ca", &address)
//! );
//! ```
//!
//! [`Identifier`]: ./struct.Identifier.html
//! [`SigningKey`]: ./struct.SigningKey.html
//!

use crate::crypto::key;
use chain_addr::{Address, Discrimination};
use chain_crypto::{AsymmetricKey, Ed25519, Ed25519Extended, SecretKey};
use chain_impl_mockchain::{
    account,
    key::{AccountPublicKey, EitherEd25519SecretKey},
};
use rand_core::{CryptoRng, RngCore};
use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};

/// Account identifier, used to identify an account. Cryptographically linked
/// to the account [`SigningKey`].
///
/// [`SigningKey`]: ./struct.SigningKey.html
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
pub struct Identifier(key::Identifier<account::AccountAlg>);

/// account Singing key. Used to sign transaction. Any owner of this key can
/// utilise the associated values or stake.
///
#[derive(Clone)]
pub struct SigningKey(EitherEd25519SecretKey);

custom_error! {pub SigningKeyParseError
    InvalidBech32Encoding { source: bech32::Error } = "Invalid bech32: {source}",
    InvalidSecretKey { source: chain_crypto::bech32::Error } = "Invalid secret key: {source}",
    UnexpectedHRP { hrp: String } = "Unexpected key '{hrp}'. Expected either ed25519 or ed25519extended",
}

impl Identifier {
    /// get the address associated to this account identifier
    #[inline]
    pub fn to_address(&self, discrimination: Discrimination) -> Address {
        self.0.to_account_address(discrimination)
    }

    /// retrieve the underlying account identifer as used in the library
    /// so you can perform the relevant operations on transaction or the
    /// ledger.
    #[inline]
    pub fn to_inner(&self) -> account::Identifier {
        account::Identifier::from(self.as_ref().clone())
    }

    #[inline]
    pub fn to_bech32_str(&self) -> String {
        self.0.to_bech32_str()
    }

    #[inline]
    pub fn from_bech32_str(s: &str) -> Result<Self, chain_crypto::bech32::Error> {
        key::Identifier::from_bech32_str(s).map(Identifier)
    }

    #[inline]
    pub fn to_hex(&self) -> String {
        self.0.to_hex()
    }

    #[inline]
    pub fn from_hex(s: &str) -> Result<Self, chain_crypto::PublicKeyFromStrError> {
        key::Identifier::from_hex(s).map(Identifier)
    }
}

impl SigningKey {
    /// get the identifier associated to this key.
    #[inline]
    pub fn identifier(&self) -> Identifier {
        Identifier(self.0.to_public().into())
    }

    /// generate a new _Account_ `SigningKey`
    ///
    /// This function will generate a _normal_ Ed25519 secret key. If you wished to use
    /// an Ed25519Extended secret key, use `generate_extended`.
    #[inline]
    pub fn generate<RNG>(rng: RNG) -> Self
    where
        RNG: RngCore + CryptoRng,
    {
        SigningKey(EitherEd25519SecretKey::Normal(SecretKey::generate(rng)))
    }

    /// generate a new _Account_ `SigningKey`
    ///
    /// This function will generate an _extended_ Ed25519 secret key. If you wished to use
    /// an Ed25519 secret key, use `generate`.
    #[inline]
    pub fn generate_extended<RNG>(rng: RNG) -> Self
    where
        RNG: RngCore + CryptoRng,
    {
        SigningKey(EitherEd25519SecretKey::Extended(SecretKey::generate(rng)))
    }

    #[inline]
    pub fn to_bech32_str(&self) -> String {
        use chain_crypto::bech32::Bech32 as _;
        match &self.0 {
            EitherEd25519SecretKey::Normal(ed25519_key) => ed25519_key.to_bech32_str(),
            EitherEd25519SecretKey::Extended(ed25519e_key) => ed25519e_key.to_bech32_str(),
        }
    }

    #[inline]
    pub fn from_bech32_str(s: &str) -> Result<Self, SigningKeyParseError> {
        use chain_crypto::bech32::Bech32 as _;

        let bech32_encoded = bech32::Bech32::from_str(s)?;

        let key = match bech32_encoded.hrp() {
            <Ed25519 as AsymmetricKey>::SECRET_BECH32_HRP => SigningKey(
                EitherEd25519SecretKey::Normal(SecretKey::try_from_bech32_str(s)?),
            ),
            <Ed25519Extended as AsymmetricKey>::SECRET_BECH32_HRP => SigningKey(
                EitherEd25519SecretKey::Extended(SecretKey::try_from_bech32_str(s)?),
            ),
            hrp => {
                return Err(SigningKeyParseError::UnexpectedHRP {
                    hrp: hrp.to_owned(),
                })
            }
        };

        Ok(key)
    }
}

/* ---------------- Display ------------------------------------------------ */

impl fmt::Debug for SigningKey {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_tuple("SigningKey").finish()
    }
}

impl fmt::Display for Identifier {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.to_bech32_str().fmt(f)
    }
}

impl FromStr for Identifier {
    type Err = chain_crypto::bech32::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_bech32_str(s)
    }
}

/* ---------------- AsRef -------------------------------------------------- */

impl AsRef<AccountPublicKey> for Identifier {
    fn as_ref(&self) -> &AccountPublicKey {
        self.0.as_ref()
    }
}

impl AsRef<EitherEd25519SecretKey> for SigningKey {
    fn as_ref(&self) -> &EitherEd25519SecretKey {
        &self.0
    }
}

/* ---------------- Conversion --------------------------------------------- */

impl From<SecretKey<Ed25519>> for SigningKey {
    fn from(key: SecretKey<Ed25519>) -> Self {
        SigningKey(EitherEd25519SecretKey::Normal(key))
    }
}

impl From<SecretKey<Ed25519Extended>> for SigningKey {
    fn from(key: SecretKey<Ed25519Extended>) -> Self {
        SigningKey(EitherEd25519SecretKey::Extended(key))
    }
}

impl From<AccountPublicKey> for Identifier {
    fn from(key: AccountPublicKey) -> Self {
        Identifier(key::Identifier::from(key))
    }
}

impl From<account::Identifier> for Identifier {
    fn from(identifier: account::Identifier) -> Self {
        Identifier(key::Identifier::from(identifier.as_ref().clone()))
    }
}

/* ------------------- Serde ----------------------------------------------- */

impl Serialize for SigningKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        if serializer.is_human_readable() {
            self.to_bech32_str().serialize(serializer)
        } else {
            unimplemented!()
        }
    }
}

impl<'de> Deserialize<'de> for SigningKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        if deserializer.is_human_readable() {
            let s = String::deserialize(deserializer)?;
            SigningKey::from_bech32_str(&s).map_err(<D::Error as serde::de::Error>::custom)
        } else {
            unimplemented!()
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use quickcheck::{Arbitrary, Gen, TestResult};

    impl Arbitrary for SigningKey {
        fn arbitrary<G>(g: &mut G) -> Self
        where
            G: Gen,
        {
            if bool::arbitrary(g) {
                let key: key::SigningKey<Ed25519> = key::SigningKey::arbitrary(g);
                SigningKey(EitherEd25519SecretKey::Normal(key.0))
            } else {
                let key: key::SigningKey<Ed25519Extended> = key::SigningKey::arbitrary(g);
                SigningKey(EitherEd25519SecretKey::Extended(key.0))
            }
        }
    }

    impl Arbitrary for Identifier {
        fn arbitrary<G>(g: &mut G) -> Self
        where
            G: Gen,
        {
            SigningKey::arbitrary(g).identifier()
        }
    }

    // test to check that account identifier is encoded in hexadecimal
    // when we use the Display trait
    #[test]
    fn identifier_display() {
        const EXPECTED_IDENTIFIER_STR: &'static str =
            "ed25519_pk1yqszqgpqyqszqgpqyqszqgpqyqszqgpqyqszqgpqyqszqgpqyqsqyl7vm8";
        const IDENTIFIER_BYTES: [u8; 32] = [0x20; 32];

        let identifier = Identifier(
            AccountPublicKey::from_binary(&IDENTIFIER_BYTES)
                .unwrap()
                .into(),
        );

        assert_eq!(identifier.to_string(), EXPECTED_IDENTIFIER_STR);
    }

    // check that the account identifier is encoded with bech32 when utilising
    // serde with a human readable output
    #[test]
    fn identifier_serde_human_readable() {
        const EXPECTED_IDENTIFIER_STR: &'static str =
            "---\ned25519_pk1yqszqgpqyqszqgpqyqszqgpqyqszqgpqyqszqgpqyqszqgpqyqsqyl7vm8";
        const IDENTIFIER_BYTES: [u8; 32] = [0x20; 32];

        let identifier = Identifier(
            AccountPublicKey::from_binary(&IDENTIFIER_BYTES)
                .unwrap()
                .into(),
        );

        let identifier_str = serde_yaml::to_string(&identifier).unwrap();

        assert_eq!(identifier_str, EXPECTED_IDENTIFIER_STR);
    }

    // check that the account signing key is encoded with bech32 when utilising
    // serde with a human readable output
    #[test]
    fn signing_key_serde_human_readable() {
        const EXPECTED_SIGNING_KEY_STR: &'static str =
            "---\ned25519e_sk1yqszqgpqyqszqgpqyqszqgpqyqszqgpqyqszqgpqyqszqgpqyqszqgpqyqszqgpqyqszqgpqyqszqgpqyqszqgpqyqszqgpqyqszqgq4hu79f";
        const SIGNING_KEY_BYTES: [u8; 64] = [0x20; 64];

        let signing_key = SigningKey(EitherEd25519SecretKey::Extended(
            SecretKey::from_binary(&SIGNING_KEY_BYTES).unwrap(),
        ));

        let signing_key_str = serde_yaml::to_string(&signing_key).unwrap();

        assert_eq!(signing_key_str, EXPECTED_SIGNING_KEY_STR);
    }

    quickcheck! {
        fn identifier_display_and_from_str(identifier: Identifier) -> TestResult {
            let identifier_str = identifier.to_string();
            let identifier_dec = match Identifier::from_str(&identifier_str) {
                Err(error) => return TestResult::error(error.to_string()),
                Ok(identifier) => identifier,
            };

            TestResult::from_bool(identifier_dec == identifier)
        }

        fn identifier_serde_human_readable_encode_decode(identifier: Identifier) -> TestResult {
            let identifier_str = serde_yaml::to_string(&identifier).unwrap();
            let identifier_dec : Identifier = match serde_yaml::from_str(&identifier_str) {
                Err(error) => return TestResult::error(error.to_string()),
                Ok(identifier) => identifier,
            };

            TestResult::from_bool(identifier_dec == identifier)
        }

        fn signing_key_serde_human_readable_encode_decode(signing_key: SigningKey) -> TestResult {
            let signing_key_str = serde_yaml::to_string(&signing_key).unwrap();
            let signing_key_dec : SigningKey = match serde_yaml::from_str(&signing_key_str) {
                Err(error) => return TestResult::error(error.to_string()),
                Ok(signing_key) => signing_key,
            };

            // here we compare the identifiers as there is no other way to compare the
            // secret key (Eq is not implemented for secret -- with reason!).
            TestResult::from_bool(signing_key_dec.identifier() == signing_key.identifier())
        }
    }
}
