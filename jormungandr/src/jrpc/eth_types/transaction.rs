use super::{bytes::Bytes, number::Number};
use chain_evm::{
    ethereum_types::{H160, H256, U256},
    transaction::EthereumUnsignedTransaction,
    AccessList,
};
use chain_impl_mockchain::evm::{EvmActionType, EvmTransaction};
use serde::{
    de::{self, Visitor},
    ser::SerializeStruct,
    Deserialize, Serialize,
};

#[derive(Debug, PartialEq, Eq)]
pub enum TransactionType {
    Legacy,
    EIP2930 {
        /// Pre-pay to warm storage access.
        access_list: AccessList,
    },
    EIP1559 {
        /// Pre-pay to warm storage access.
        access_list: AccessList,
        /// Max BaseFeePerGas the user is willing to pay.
        max_fee_per_gas: Number,
        /// The miner's tip.
        max_priority_fee_per_gas: Number,
    },
}

impl Serialize for TransactionType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("TransactionType", 4)?;
        match self {
            TransactionType::Legacy => {
                state.serialize_field("type", &Number::from(0))?;
            }
            TransactionType::EIP2930 { access_list } => {
                state.serialize_field("type", &Number::from(1))?;
                state.serialize_field("accessList", &access_list)?;
            }
            TransactionType::EIP1559 {
                access_list,
                max_fee_per_gas,
                max_priority_fee_per_gas,
            } => {
                state.serialize_field("type", &Number::from(2))?;
                state.serialize_field("accessList", &access_list)?;
                state.serialize_field("maxFeePerGas", &max_fee_per_gas)?;
                state.serialize_field("maxPriorityFeePerGas", &max_priority_fee_per_gas)?;
            }
        };
        state.end()
    }
}

impl<'de> Deserialize<'de> for TransactionType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(TransactionTypeVisitor)
    }
}

struct TransactionTypeVisitor;

impl<'de> Visitor<'de> for TransactionTypeVisitor {
    type Value = TransactionType;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "TransactionType representation")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        const FIELDS: &[&str] = &["type", "accessList", "maxFeePerGas", "maxPriorityFeePerGas"];

        let mut id: Option<Number> = None;
        let mut access_list = None;
        let mut max_fee_per_gas = None;
        let mut max_priority_fee_per_gas = None;

        while let Some(key) = map.next_key()? {
            match key {
                "type" => {
                    if id.is_some() {
                        return Err(de::Error::duplicate_field("type"));
                    }
                    id = Some(map.next_value()?);
                }
                "accessList" => {
                    if access_list.is_some() {
                        return Err(de::Error::duplicate_field("accessList"));
                    }
                    access_list = Some(map.next_value()?);
                }
                "maxFeePerGas" => {
                    if max_fee_per_gas.is_some() {
                        return Err(de::Error::duplicate_field("maxFeePerGas"));
                    }
                    max_fee_per_gas = Some(map.next_value()?);
                }
                "maxPriorityFeePerGas" => {
                    if max_priority_fee_per_gas.is_some() {
                        return Err(de::Error::duplicate_field("maxPriorityFeePerGas"));
                    }
                    max_priority_fee_per_gas = Some(map.next_value()?);
                }
                value => return Err(de::Error::unknown_field(value, FIELDS)),
            }
        }

        match id {
            Some(id) if id == 0.into() => Ok(Self::Value::Legacy),
            Some(id) if id == 1.into() => match access_list {
                Some(access_list) => Ok(Self::Value::EIP2930 { access_list }),
                None => Err(de::Error::missing_field("accessList")),
            },
            Some(id) if id == 2.into() => match access_list {
                Some(access_list) => match max_fee_per_gas {
                    Some(max_fee_per_gas) => match max_priority_fee_per_gas {
                        Some(max_priority_fee_per_gas) => Ok(Self::Value::EIP1559 {
                            access_list,
                            max_fee_per_gas,
                            max_priority_fee_per_gas,
                        }),
                        None => Err(de::Error::missing_field("maxPriorityFeePerGas")),
                    },
                    None => Err(de::Error::missing_field("maxFeePerGas")),
                },
                None => Err(de::Error::missing_field("accessList")),
            },
            Some(_) => Err(de::Error::custom(
                "invalid type value, should be 0 or 1 or 2",
            )),
            None => Err(de::Error::missing_field("type")),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Transaction {
    /// Nonce
    nonce: Number,
    /// Sender
    pub(crate) from: H160,
    /// Recipient
    to: Option<H160>,
    /// Transfered value
    value: Number,
    /// Gas
    gas: Number,
    /// Data
    input: Bytes,
    /// Gas price
    gas_price: Number,
    /// The network id of the transaction, if any.
    chain_id: Option<Number>,
    /// Block hash, null when pending
    block_hash: Option<H256>,
    /// Block number, null when pending
    block_number: Option<Number>,
    /// Transaction Index, null when pending
    transaction_index: Option<Number>,
    /// The standardised V field of the signature.
    v: Number,
    /// The R field of the signature.
    r: U256,
    /// The S field of the signature.
    s: U256,
    #[serde(flatten)]
    transaction_type: TransactionType,
}

impl From<Transaction> for EthereumUnsignedTransaction {
    fn from(_other: Transaction) -> Self {
        unimplemented!("this is left as pending for future work");
    }
}

impl From<Transaction> for EvmTransaction {
    fn from(val: Transaction) -> Self {
        let caller = val.from;
        let value = val.value.into();
        let nonce = val.nonce.into();
        let gas_limit = val.gas_price.into();
        let access_list = match val.transaction_type {
            TransactionType::Legacy => Vec::new(),
            TransactionType::EIP2930 { access_list } => access_list,
            TransactionType::EIP1559 { access_list, .. } => access_list,
        };

        match val.to {
            Some(address) => Self {
                caller,
                value,
                nonce,
                gas_limit,
                access_list,
                action_type: EvmActionType::Call {
                    address,
                    data: val.input.into(),
                },
            },
            None => Self {
                caller,
                value,
                nonce,
                gas_limit,
                access_list,
                action_type: EvmActionType::Create {
                    init_code: val.input.into(),
                },
            },
        }
    }
}

impl Transaction {
    pub fn build(
        tx: EvmTransaction,
        block_hash: Option<H256>,
        block_number: Option<Number>,
        transaction_index: Option<Number>,
        gas_price: u64,
    ) -> Self {
        match tx.action_type {
            EvmActionType::Call { address, data } => Self {
                block_hash,
                block_number,
                nonce: tx.nonce.into(),
                from: tx.caller,
                to: Some(address),
                value: tx.value.into(),
                gas: tx.gas_limit.into(),
                input: data.into(),
                gas_price: gas_price.into(),
                chain_id: Some(1.into()),
                transaction_index,
                v: 1.into(),
                r: U256::one(),
                s: U256::one(),
                transaction_type: TransactionType::EIP2930 {
                    access_list: tx.access_list,
                },
            },
            EvmActionType::Create { init_code } => Self {
                block_hash,
                block_number,
                nonce: tx.nonce.into(),
                from: tx.caller,
                to: None,
                value: tx.value.into(),
                gas: tx.gas_limit.into(),
                input: init_code.into(),
                gas_price: gas_price.into(),
                chain_id: Some(1.into()),
                transaction_index,
                v: 1.into(),
                r: U256::one(),
                s: U256::one(),
                transaction_type: TransactionType::EIP2930 {
                    access_list: tx.access_list,
                },
            },
            EvmActionType::Create2 { init_code, salt: _ } => Self {
                block_hash,
                block_number,
                nonce: tx.nonce.into(),
                from: tx.caller,
                to: None,
                value: tx.value.into(),
                gas: tx.gas_limit.into(),
                input: init_code.into(),
                gas_price: gas_price.into(),
                chain_id: Some(1.into()),
                transaction_index,
                v: 1.into(),
                r: U256::one(),
                s: U256::one(),
                transaction_type: TransactionType::EIP2930 {
                    access_list: tx.access_list,
                },
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transaction_json_serde() {
        let legacy_transaction = Transaction {
            nonce: 0.into(),
            from: H160::zero(),
            to: Some(H160::zero()),
            value: 0.into(),
            gas: 0.into(),
            input: Default::default(),
            gas_price: 0.into(),
            chain_id: Some(0.into()),
            block_hash: None,
            block_number: None,
            transaction_index: None,
            v: 0.into(),
            r: U256::zero(),
            s: U256::zero(),
            transaction_type: TransactionType::Legacy,
        };
        assert_eq!(
            serde_json::to_string(&legacy_transaction).unwrap(),
            r#"{"nonce":"0x0","from":"0x0000000000000000000000000000000000000000","to":"0x0000000000000000000000000000000000000000","value":"0x0","gas":"0x0","input":"0x","gasPrice":"0x0","chainId":"0x0","blockHash":null,"blockNumber":null,"transactionIndex":null,"v":"0x0","r":"0x0","s":"0x0","type":"0x0"}"#
        );
        let legacy_decoded: Transaction = serde_json::from_str(r#"{"blockHash":null,"blockNumber":null,"nonce":"0x0","from":"0x0000000000000000000000000000000000000000","to":"0x0000000000000000000000000000000000000000","value":"0x0","gas":"0x0","input":"0x","gasPrice":"0x0","chainId":"0x0","transactionIndex":null,"v":"0x0","r":"0x0","s":"0x0","type":"0x0"}"#).unwrap();
        assert_eq!(legacy_decoded, legacy_transaction);

        let eip2930_transaction = Transaction {
            nonce: 0.into(),
            from: H160::zero(),
            to: Some(H160::zero()),
            value: 0.into(),
            gas: 0.into(),
            input: Default::default(),
            gas_price: 0.into(),
            chain_id: Some(0.into()),
            block_hash: None,
            block_number: None,
            transaction_index: None,
            v: 0.into(),
            r: U256::zero(),
            s: U256::zero(),
            transaction_type: TransactionType::EIP2930 {
                access_list: Vec::new(),
            },
        };
        assert_eq!(
            serde_json::to_string(&eip2930_transaction).unwrap(),
            r#"{"nonce":"0x0","from":"0x0000000000000000000000000000000000000000","to":"0x0000000000000000000000000000000000000000","value":"0x0","gas":"0x0","input":"0x","gasPrice":"0x0","chainId":"0x0","blockHash":null,"blockNumber":null,"transactionIndex":null,"v":"0x0","r":"0x0","s":"0x0","type":"0x1","accessList":[]}"#
        );
        let eip1559_decoded: Transaction = serde_json::from_str(r#"{"accessList":[],"blockHash":null,"blockNumber":null,"nonce":"0x0","from":"0x0000000000000000000000000000000000000000","to":"0x0000000000000000000000000000000000000000","value":"0x0","gas":"0x0","input":"0x","gasPrice":"0x0","chainId":"0x0","transactionIndex":null,"v":"0x0","r":"0x0","s":"0x0","type":"0x1"}"#).unwrap();
        assert_eq!(eip1559_decoded, eip2930_transaction);

        let eip1559_transaction = Transaction {
            nonce: 0.into(),
            from: H160::zero(),
            to: Some(H160::zero()),
            value: 0.into(),
            gas: 0.into(),
            input: Default::default(),
            gas_price: 0.into(),
            chain_id: Some(0.into()),
            block_hash: None,
            block_number: None,
            transaction_index: None,
            v: 0.into(),
            r: U256::zero(),
            s: U256::zero(),
            transaction_type: TransactionType::EIP1559 {
                access_list: Vec::new(),
                max_fee_per_gas: 0.into(),
                max_priority_fee_per_gas: 0.into(),
            },
        };
        assert_eq!(
            serde_json::to_string(&eip1559_transaction).unwrap(),
            r#"{"nonce":"0x0","from":"0x0000000000000000000000000000000000000000","to":"0x0000000000000000000000000000000000000000","value":"0x0","gas":"0x0","input":"0x","gasPrice":"0x0","chainId":"0x0","blockHash":null,"blockNumber":null,"transactionIndex":null,"v":"0x0","r":"0x0","s":"0x0","type":"0x2","accessList":[],"maxFeePerGas":"0x0","maxPriorityFeePerGas":"0x0"}"#
        );
        let eip1559_decoded: Transaction = serde_json::from_str(r#"{"maxFeePerGas":"0x0","maxPriorityFeePerGas":"0x0","accessList":[],"blockHash":null,"blockNumber":null,"nonce":"0x0","from":"0x0000000000000000000000000000000000000000","to":"0x0000000000000000000000000000000000000000","value":"0x0","gas":"0x0","input":"0x","gasPrice":"0x0","chainId":"0x0","transactionIndex":null,"v":"0x0","r":"0x0","s":"0x0","type":"0x2"}"#).unwrap();
        assert_eq!(eip1559_decoded, eip1559_transaction);

        let legacy_decoded_without_nulls: Transaction = serde_json::from_str(r#"{"nonce":"0x0","from":"0x0000000000000000000000000000000000000000","to":"0x0000000000000000000000000000000000000000","value":"0x0","gas":"0x0","input":"0x","gasPrice":"0x0","chainId":"0x0","v":"0x0","r":"0x0","s":"0x0","type":"0x0"}"#).unwrap();
        assert_eq!(legacy_decoded_without_nulls, legacy_decoded);
    }
}
