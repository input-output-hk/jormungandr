use chain_impl_mockchain::transaction::Witness;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{fmt, str::FromStr};

custom_error! {pub TransactionWitnessFromStrError
    Bech32 { source: bech32::Error } = "Invalid bech32 encoding",
    InvalidHrp { expected: String, got: String } = "Invalid prefix, expected '{expected}' but received '{got}'",
    Invalid { source: chain_core::mempack::ReadError } = "Invalid encoding",
}

const HRP: &'static str = "witness";

/// a transaction witness
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransactionWitness(Witness);

impl TransactionWitness {
    pub fn to_bech32_str(&self) -> String {
        use bech32::{Bech32, ToBase32 as _};
        use chain_core::property::Serialize as _;

        let bytes = self.as_ref().serialize_as_vec().unwrap();

        let bech32 = Bech32::new(HRP.to_owned(), bytes.to_base32()).unwrap();
        bech32.to_string()
    }

    pub fn from_bech32_str(s: &str) -> Result<Self, TransactionWitnessFromStrError> {
        use bech32::{Bech32, FromBase32};
        use chain_core::mempack::{ReadBuf, Readable as _};

        let bech32: Bech32 = s.parse()?;
        if bech32.hrp() != HRP {
            return Err(TransactionWitnessFromStrError::InvalidHrp {
                expected: HRP.to_owned(),
                got: bech32.hrp().to_owned(),
            });
        }
        let bytes = Vec::<u8>::from_base32(bech32.data())?;

        let mut reader = ReadBuf::from(&bytes);
        Ok(Witness::read(&mut reader).map(TransactionWitness)?)
    }
}

/* ---------------- Display ------------------------------------------------ */

impl fmt::Display for TransactionWitness {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.to_bech32_str().fmt(f)
    }
}

impl FromStr for TransactionWitness {
    type Err = TransactionWitnessFromStrError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_bech32_str(s)
    }
}

/* ---------------- AsRef -------------------------------------------------- */

impl AsRef<Witness> for TransactionWitness {
    fn as_ref(&self) -> &Witness {
        &self.0
    }
}
/* ---------------- Conversion --------------------------------------------- */

impl From<Witness> for TransactionWitness {
    fn from(v: Witness) -> Self {
        TransactionWitness(v)
    }
}

impl From<TransactionWitness> for Witness {
    fn from(v: TransactionWitness) -> Self {
        v.0
    }
}

/* ------------------- Serde ----------------------------------------------- */

impl Serialize for TransactionWitness {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use chain_core::property::Serialize as _;
        use serde::ser::Error as _;

        let bytes = self
            .as_ref()
            .serialize_as_vec()
            .map_err(|err| S::Error::custom(err))?;

        if serializer.is_human_readable() {
            use bech32::{Bech32, ToBase32 as _};
            let bech32 = Bech32::new(HRP.to_owned(), bytes.to_base32())
                .map_err(|err| S::Error::custom(err))?;
            serializer.serialize_str(&bech32.to_string())
        } else {
            serializer.serialize_bytes(&bytes)
        }
    }
}

impl<'de> Deserialize<'de> for TransactionWitness {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use chain_core::mempack::{ReadBuf, Readable as _};
        use serde::de::{self, Visitor};
        struct TransactionWitnessVisitor;
        impl<'de> Visitor<'de> for TransactionWitnessVisitor {
            type Value = TransactionWitness;
            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a transaction witness")
            }

            fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                use bech32::{Bech32, FromBase32};
                let bech32: Bech32 = s.parse().map_err(E::custom)?;
                if bech32.hrp() != HRP {
                    return Err(E::custom(format!(
                        "Invalid prefix: expected '{}' but was '{}'",
                        HRP,
                        bech32.hrp()
                    )));
                }
                let bytes = Vec::<u8>::from_base32(bech32.data())
                    .map_err(|err| E::custom(format!("Invalid bech32: {}", err)))?;
                self.visit_bytes(&bytes)
            }

            fn visit_bytes<E>(self, bytes: &[u8]) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                let mut reader = ReadBuf::from(bytes);
                Witness::read(&mut reader)
                    .map_err(E::custom)
                    .map(TransactionWitness)
            }
        }

        if deserializer.is_human_readable() {
            deserializer.deserialize_str(TransactionWitnessVisitor)
        } else {
            deserializer.deserialize_bytes(TransactionWitnessVisitor)
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use quickcheck::{Arbitrary, Gen, TestResult};

    impl Arbitrary for TransactionWitness {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            match u8::arbitrary(g) % 3 {
                0 => Witness::Utxo(Arbitrary::arbitrary(g)).into(),
                1 => Witness::Account(Arbitrary::arbitrary(g)).into(),
                2 => {
                    use crate::crypto::key::KeyPair;
                    use chain_crypto::Ed25519Bip32;
                    let kp: KeyPair<Ed25519Bip32> = KeyPair::arbitrary(g);
                    Witness::OldUtxo(kp.identifier().as_ref().clone(), Arbitrary::arbitrary(g))
                        .into()
                }
                3 => unimplemented!(), // Multisig
                _ => unreachable!(),
            }
        }
    }


    quickcheck! {
        fn identifier_display_and_from_str(transaction_witness: TransactionWitness) -> TestResult {
            let transaction_witness_str = transaction_witness.to_string();
            let transaction_witness_dec = match TransactionWitness::from_str(&transaction_witness_str) {
                Err(error) => return TestResult::error(error.to_string()),
                Ok(transaction_witness) => transaction_witness,
            };

            TestResult::from_bool(transaction_witness_dec == transaction_witness)
        }

        fn transaction_witness_serde_human_readable_encode_decode(transaction_witness: TransactionWitness) -> TestResult {
            let transaction_witness_str = serde_yaml::to_string(&transaction_witness).unwrap();
            let transaction_witness_dec : TransactionWitness = match serde_yaml::from_str(&transaction_witness_str) {
                Err(error) => return TestResult::error(error.to_string()),
                Ok(transaction_witness) => transaction_witness,
            };

            TestResult::from_bool(transaction_witness_dec == transaction_witness)
        }

        fn transaction_witness_serde_bincode_readable_encode_decode(transaction_witness: TransactionWitness) -> TestResult {
            let transaction_witness_bytes = bincode::serialize(&transaction_witness).unwrap();
            let transaction_witness_dec : TransactionWitness = match bincode::deserialize(&transaction_witness_bytes) {
                Err(error) => return TestResult::error(error.to_string()),
                Ok(transaction_witness) => transaction_witness,
            };

            TestResult::from_bool(transaction_witness_dec == transaction_witness)
        }
    }
}
