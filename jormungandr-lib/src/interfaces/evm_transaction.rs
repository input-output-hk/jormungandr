use chain_core::mempack::{ReadBuf, Readable};
use chain_impl_mockchain::evm;
use serde::{Deserialize, Serialize};
use typed_bytes::ByteBuilder;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvmTransaction(evm::EvmTransaction);

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

impl Serialize for EvmTransaction {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let data = self.0.serialize_in(ByteBuilder::new()).finalize_as_vec();
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
                evm::EvmTransaction::read(&mut ReadBuf::from(&data))
                    .map_err(<D::Error as serde::de::Error>::custom)?,
            ))
        } else {
            let data = <Vec<u8>>::deserialize(deserializer)
                .map_err(<D::Error as serde::de::Error>::custom)?;
            Ok(Self(
                evm::EvmTransaction::read(&mut ReadBuf::from(&data))
                    .map_err(<D::Error as serde::de::Error>::custom)?,
            ))
        }
    }
}

#[cfg(test)]
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
