use crate::interfaces::Address;
use chain_impl_mockchain::transaction;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{fmt, str::FromStr};
use thiserror::Error;

const DEFAULT_DISCRIMINATION: chain_addr::Discrimination = chain_addr::Discrimination::Production;
const DEFAULT_PREFIX: &str = "identifier";

/// An account identifier for the different kind of accounts
/// (single or multi).
///
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AccountIdentifier(transaction::AccountIdentifier);

impl AccountIdentifier {
    pub fn to_address(self, discrimination: chain_addr::Discrimination, prefix: &str) -> Address {
        let kind = match self.0 {
            transaction::AccountIdentifier::Single(identifier) => {
                let public_key = identifier.into();
                chain_addr::Kind::Account(public_key)
            }
            transaction::AccountIdentifier::Multi(identifier) => {
                let key = identifier.into();
                chain_addr::Kind::Multisig(key)
            }
        };

        let addr = chain_addr::Address(discrimination, kind);
        Address(prefix.to_owned(), addr)
    }

    fn from_address(addr: Address) -> Result<Self, ParseAccountIdentifierError> {
        let kind = match (addr.1).1 {
            chain_addr::Kind::Account(pk) => transaction::AccountIdentifier::Single(pk.into()),
            chain_addr::Kind::Multisig(id) => transaction::AccountIdentifier::Multi(id.into()),
            _ => return Err(ParseAccountIdentifierError::NotAccountOrMulti),
        };
        Ok(AccountIdentifier(kind))
    }
}

/* ---------------- Display ------------------------------------------------ */

impl fmt::Display for AccountIdentifier {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.clone()
            .to_address(DEFAULT_DISCRIMINATION, DEFAULT_PREFIX)
            .fmt(f)
    }
}

#[derive(Debug, Error)]
pub enum ParseAccountIdentifierError {
    #[error("Cannot parse account identifier")]
    CannotParseAddress {
        #[source]
        #[from]
        source: chain_addr::Error,
    },

    #[error("Invalid account identifier, expected single account or multisig account")]
    NotAccountOrMulti,
}

impl FromStr for AccountIdentifier {
    type Err = ParseAccountIdentifierError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let addr: chain_addr::Address = Address::from_str(s)?.into();
        let kind = match addr.1 {
            chain_addr::Kind::Account(pk) => transaction::AccountIdentifier::Single(pk.into()),
            chain_addr::Kind::Multisig(id) => transaction::AccountIdentifier::Multi(id.into()),
            _ => return Err(ParseAccountIdentifierError::NotAccountOrMulti),
        };

        Ok(AccountIdentifier(kind))
    }
}

/* ---------------- AsRef -------------------------------------------------- */

impl AsRef<transaction::AccountIdentifier> for AccountIdentifier {
    fn as_ref(&self) -> &transaction::AccountIdentifier {
        &self.0
    }
}
/* ---------------- Conversion --------------------------------------------- */

impl From<transaction::AccountIdentifier> for AccountIdentifier {
    fn from(v: transaction::AccountIdentifier) -> Self {
        AccountIdentifier(v)
    }
}

impl From<AccountIdentifier> for transaction::AccountIdentifier {
    fn from(v: AccountIdentifier) -> Self {
        v.0
    }
}

/* ------------------- Serde ----------------------------------------------- */

impl Serialize for AccountIdentifier {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.clone()
            .to_address(DEFAULT_DISCRIMINATION, DEFAULT_PREFIX)
            .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for AccountIdentifier {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::Error as _;
        Address::deserialize(deserializer)
            .and_then(|addr| Self::from_address(addr).map_err(D::Error::custom))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::crypto::key::KeyPair;
    use quickcheck::{Arbitrary, Gen, TestResult};

    impl Arbitrary for AccountIdentifier {
        fn arbitrary<G>(g: &mut G) -> Self
        where
            G: Gen,
        {
            let kind = match u8::arbitrary(g) % 2 {
                0 => {
                    let kp: KeyPair<chain_crypto::Ed25519> = KeyPair::arbitrary(g);
                    let pk: chain_crypto::PublicKey<chain_crypto::Ed25519> =
                        kp.identifier().into_public_key();

                    transaction::AccountIdentifier::Single(pk.into())
                }
                1 => {
                    let mut bytes = [0; 32];
                    for byte in bytes.iter_mut() {
                        *byte = u8::arbitrary(g);
                    }
                    transaction::AccountIdentifier::Multi(bytes.into())
                }
                _ => unreachable!(),
            };

            AccountIdentifier(kind)
        }
    }

    quickcheck! {
        fn address_display_parse(address: AccountIdentifier) -> TestResult {
            let s = address.to_string();
            let address_dec: AccountIdentifier = s.parse().unwrap();

            TestResult::from_bool(address == address_dec)
        }

        fn address_serde_human_readable_encode_decode(address: AccountIdentifier) -> TestResult {
            let s = serde_yaml::to_string(&address).unwrap();
            let address_dec: AccountIdentifier = serde_yaml::from_str(&s).unwrap();

            TestResult::from_bool(address == address_dec)
        }

        fn address_serde_binary_encode_decode(address: AccountIdentifier) -> TestResult {
            let s = bincode::serialize(&address).unwrap();
            let address_dec: AccountIdentifier = bincode::deserialize(&s).unwrap();

            TestResult::from_bool(address == address_dec)
        }
    }
}
