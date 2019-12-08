use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{fmt, str::FromStr};

/// Address with the appropriate implementation for Serde API and
/// Display/FromStr interfaces.
///
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Address(pub(crate) String, pub(crate) chain_addr::Address);

/* ---------------- Display ------------------------------------------------ */

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        chain_addr::AddressReadable::from_address(&self.0, &self.1).fmt(f)
    }
}

impl FromStr for Address {
    type Err = chain_addr::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse()
            .map(|v: chain_addr::AddressReadable| Address(v.get_prefix(), v.to_address()))
    }
}

/* ---------------- AsRef -------------------------------------------------- */

impl AsRef<chain_addr::Address> for Address {
    fn as_ref(&self) -> &chain_addr::Address {
        &self.1
    }
}
/* ---------------- Conversion --------------------------------------------- */

impl From<chain_addr::Address> for Address {
    fn from(v: chain_addr::Address) -> Self {
        Address("ca".to_owned(), v)
    }
}

impl From<Address> for chain_addr::Address {
    fn from(v: Address) -> Self {
        v.1
    }
}

/* ------------------- Serde ----------------------------------------------- */

impl Serialize for Address {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if serializer.is_human_readable() {
            let address = self.to_string();
            serializer.serialize_str(&address)
        } else {
            let bytes = self.1.to_bytes();
            serializer.serialize_bytes(&bytes)
        }
    }
}

impl<'de> Deserialize<'de> for Address {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        if deserializer.is_human_readable() {
            let s: String = String::deserialize(deserializer)?;
            chain_addr::AddressReadable::from_string_anyprefix(&s)
                .map_err(|e| serde::de::Error::custom(e))
                .map(|a| Address(a.get_prefix(), a.to_address()))
        } else {
            let b: Vec<u8> = Vec::deserialize(deserializer)?;
            chain_addr::Address::from_bytes(&b)
                .map_err(|e| serde::de::Error::custom(e))
                .map(Address::from)
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::crypto::key::KeyPair;
    use quickcheck::{Arbitrary, Gen, TestResult};

    impl Arbitrary for Address {
        fn arbitrary<G>(g: &mut G) -> Self
        where
            G: Gen,
        {
            let kp: KeyPair<chain_crypto::Ed25519> = KeyPair::arbitrary(g);
            let pk: chain_crypto::PublicKey<chain_crypto::Ed25519> =
                kp.identifier().into_public_key();

            let (discrimination, prefix) = match bool::arbitrary(g) {
                true => (chain_addr::Discrimination::Production, "ca".to_owned()),
                false => (chain_addr::Discrimination::Test, "ca".to_owned()),
            };

            let kind = match u8::arbitrary(g) % 3 {
                0 => chain_addr::Kind::Single(pk),
                1 => chain_addr::Kind::Account(pk),
                2 => chain_addr::Kind::Group(pk.clone(), pk),
                _ => unreachable!(),
            };

            let address = chain_addr::Address(discrimination, kind);

            Address(prefix, address)
        }
    }

    quickcheck! {
        fn address_display_parse(address: Address) -> TestResult {
            let s = address.to_string();
            let address_dec: Address = s.parse().unwrap();

            TestResult::from_bool(address == address_dec)
        }

        fn address_serde_human_readable_encode_decode(address: Address) -> TestResult {
            let s = serde_yaml::to_string(&address).unwrap();
            let address_dec: Address = serde_yaml::from_str(&s).unwrap();

            TestResult::from_bool(address == address_dec)
        }

        fn address_serde_binary_encode_decode(address: Address) -> TestResult {
            let s = bincode::serialize(&address).unwrap();
            let address_dec: Address = bincode::deserialize(&s).unwrap();

            TestResult::from_bool(address == address_dec)
        }
    }
}
