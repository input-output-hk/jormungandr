//! Hash interface for all that is a hash

use crate::crypto::serde as internal;
use chain_crypto::hash::Blake2b256;
use chain_impl_mockchain::key;
use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};

/// wrapper around the Blake2b256 hash
///
#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Hash, Serialize, Deserialize)]
pub struct Hash(
    #[serde(
        deserialize_with = "internal::deserialize_hash",
        serialize_with = "internal::serialize_hash"
    )]
    Blake2b256,
);

impl Hash {
    #[inline]
    pub fn into_hash(self) -> key::Hash {
        key::Hash::from(self.0)
    }

    pub fn to_hex(&self) -> String {
        self.to_string()
    }

    pub fn from_hex(s: &str) -> Result<Self, chain_crypto::hash::Error> {
        s.parse().map(Hash)
    }
}

/* ---------------- Display ------------------------------------------------ */

impl fmt::Display for Hash {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl FromStr for Hash {
    type Err = chain_crypto::hash::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Blake2b256::from_str(s).map(Hash::from)
    }
}

impl fmt::Debug for Hash {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Hash")
            .field("0", &self.to_string())
            .finish()
    }
}

/* ---------------- AsRef -------------------------------------------------- */

impl AsRef<Blake2b256> for Hash {
    fn as_ref(&self) -> &Blake2b256 {
        &self.0
    }
}

/* ---------------- Conversion --------------------------------------------- */

impl From<Blake2b256> for Hash {
    fn from(hash: Blake2b256) -> Self {
        Hash(hash)
    }
}

impl From<key::Hash> for Hash {
    fn from(hash: key::Hash) -> Self {
        let bytes: [u8; 32] = hash.into();
        Hash(Blake2b256::from(bytes))
    }
}

impl From<[u8; 32]> for Hash {
    fn from(bytes: [u8; 32]) -> Self {
        Hash(bytes.into())
    }
}

impl From<Hash> for [u8; 32] {
    fn from(hash: Hash) -> [u8; 32] {
        hash.0.into()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use quickcheck::{Arbitrary, Gen, TestResult};

    impl Arbitrary for Hash {
        fn arbitrary<G>(g: &mut G) -> Self
        where
            G: Gen,
        {
            Hash(Blake2b256::arbitrary(g))
        }
    }

    // test to check that hash is encoded in hexadecimal
    // when we use the Display trait
    #[test]
    fn hash_display() {
        const EXPECTED_HASH_STR: &'static str =
            "2020202020202020202020202020202020202020202020202020202020202020";
        const HASH_BYTES: [u8; 32] = [0x20; 32];

        let hash = Hash(Blake2b256::from(HASH_BYTES));

        assert_eq!(hash.to_string(), EXPECTED_HASH_STR);
    }

    // check that the hash is encoded with hexadecimal when utilising
    // serde with a human readable output
    #[test]
    fn hash_serde_human_readable() {
        const EXPECTED_HASH_STR: &'static str =
            "---\n\"2020202020202020202020202020202020202020202020202020202020202020\"";
        const HASH_BYTES: [u8; 32] = [0x20; 32];

        let hash = Hash(Blake2b256::from(HASH_BYTES));

        let hash_str = serde_yaml::to_string(&hash).unwrap();

        assert_eq!(hash_str, EXPECTED_HASH_STR);
    }

    quickcheck! {
        fn hash_display_and_from_str(hash: Hash) -> TestResult {
            let hash_str = hash.to_string();
            let hash_dec = match Hash::from_str(&hash_str) {
                Err(error) => return TestResult::error(error.to_string()),
                Ok(hash) => hash,
            };

            TestResult::from_bool(hash_dec == hash)
        }

        fn hash_serde_human_readable_encode_decode(hash: Hash) -> TestResult {
            let hash_str = serde_yaml::to_string(&hash).unwrap();
            let hash_dec : Hash= match serde_yaml::from_str(&hash_str) {
                Err(error) => return TestResult::error(error.to_string()),
                Ok(hash) => hash,
            };

            TestResult::from_bool(hash_dec == hash)
        }
    }
}
