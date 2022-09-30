use chain_core::{
    packer::Codec,
    property::{DeserializeFromSlice, ReadError, Serialize as _},
};
use chain_impl_mockchain::evm;
use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};
use thiserror::Error;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvmTransaction(pub evm::EvmTransaction);

#[derive(Debug, Error)]
pub enum EvmTransactionFromStrError {
    #[error("Invalid evm transaction")]
    InvalidEvmTransaction(#[from] ReadError),
    #[error("expected evm transaction in hex")]
    InvalidHex(#[from] hex::FromHexError),
}

impl From<evm::EvmTransaction> for EvmTransaction {
    fn from(val: evm::EvmTransaction) -> Self {
        Self(val)
    }
}

impl From<EvmTransaction> for evm::EvmTransaction {
    fn from(val: EvmTransaction) -> Self {
        val.0
    }
}

impl fmt::Display for EvmTransaction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            hex::encode(self.0.serialize_as_vec().map_err(|_| fmt::Error)?)
        )
    }
}

impl FromStr for EvmTransaction {
    type Err = EvmTransactionFromStrError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let data = hex::decode(s)?;
        Ok(Self(evm::EvmTransaction::deserialize_from_slice(
            &mut Codec::new(data.as_slice()),
        )?))
    }
}

impl Serialize for EvmTransaction {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let data = self
            .0
            .serialize_as_vec()
            .map_err(|e| serde::ser::Error::custom(e.to_string()))?;
        if serializer.is_human_readable() {
            hex::encode(data).serialize(serializer)
        } else {
            data.serialize(serializer)
        }
    }
}

impl<'de> Deserialize<'de> for EvmTransaction {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        if deserializer.is_human_readable() {
            let s = String::deserialize(deserializer)?;
            let data = hex::decode(&s).map_err(<D::Error as serde::de::Error>::custom)?;
            Ok(Self(
                evm::EvmTransaction::deserialize_from_slice(&mut Codec::new(data.as_slice()))
                    .map_err(<D::Error as serde::de::Error>::custom)?,
            ))
        } else {
            let data = <Vec<u8>>::deserialize(deserializer)
                .map_err(<D::Error as serde::de::Error>::custom)?;
            Ok(Self(
                evm::EvmTransaction::deserialize_from_slice(&mut Codec::new(data.as_slice()))
                    .map_err(<D::Error as serde::de::Error>::custom)?,
            ))
        }
    }
}

#[cfg(all(test, feature = "evm"))]
mod test {
    use super::*;
    use quickcheck::Arbitrary;

    impl Arbitrary for EvmTransaction {
        fn arbitrary<G: quickcheck::Gen>(g: &mut G) -> Self {
            Self(evm::EvmTransaction::arbitrary(g))
        }
    }

    quickcheck! {
        fn evm_transaction_bincode_serde_test(evm_transaction: EvmTransaction) -> bool {
            let decoded_evm_transaction: EvmTransaction = bincode::deserialize(bincode::serialize(&evm_transaction).unwrap().as_slice()).unwrap();
            decoded_evm_transaction == evm_transaction
        }

        fn evm_transaction_yaml_serde_test(evm_transaction: EvmTransaction) -> bool {
            let decoded_evm_transaction: EvmTransaction = serde_yaml::from_str(&serde_yaml::to_string(&evm_transaction).unwrap()).unwrap();
            decoded_evm_transaction == evm_transaction
        }
    }
}
