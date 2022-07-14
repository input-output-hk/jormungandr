use super::{block_number::BlockNumber, log::Log};
use chain_evm::ethereum_types::{H160, H256};
use serde::{Deserialize, Serialize, Serializer};

/// Variadic value
#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum VariadicValue<T> {
    /// Single
    Single(T),
    /// List
    Multiple(Vec<T>),
    /// None
    Null,
}

/// Filter Address
pub type FilterAddress = VariadicValue<H160>;
/// Topic, supports `A` | `null` | `[A,B,C]` | `[A,[B,C]]` | [null,[B,C]] | [null,[null,C]]
pub type Topic = VariadicValue<VariadicValue<H256>>;

/// Filter
#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct Filter {
    /// From Block
    from_block: Option<BlockNumber>,
    /// To Block
    to_block: Option<BlockNumber>,
    /// Address
    address: FilterAddress,
    /// Topics
    topics: Topic,
    /// Block hash
    block_hash: Option<H256>,
}

/// Results of the filter_changes RPC.
#[derive(Debug, PartialEq, Eq)]
pub enum FilterChanges {
    #[allow(dead_code)]
    /// New logs.
    Logs(Vec<Log>),
    #[allow(dead_code)]
    /// New hashes (block or transactions)
    Hashes(Vec<H256>),
    /// Empty result,
    #[allow(dead_code)]
    Empty,
}

impl Serialize for FilterChanges {
    fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match *self {
            FilterChanges::Logs(ref logs) => logs.serialize(s),
            FilterChanges::Hashes(ref hashes) => hashes.serialize(s),
            FilterChanges::Empty => (&[] as &[serde_json::Value]).serialize(s),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_address_deserialize() {
        let fa_single: FilterAddress =
            serde_json::from_str(r#""0x0000000000000000000000000000000000000000""#).unwrap();
        let fa_multiple: FilterAddress =
            serde_json::from_str(r#"["0x0000000000000000000000000000000000000000"]"#).unwrap();
        let fa_null: FilterAddress = serde_json::from_str(r#"null"#).unwrap();

        assert_eq!(fa_single, FilterAddress::Single(H160::zero()));
        assert_eq!(fa_multiple, FilterAddress::Multiple(vec![H160::zero()]));
        assert_eq!(fa_null, FilterAddress::Null);
    }

    #[test]
    fn topic_deserialize() {
        let t_single_single: Topic = serde_json::from_str(
            r#""0x0000000000000000000000000000000000000000000000000000000000000000""#,
        )
        .unwrap();
        let t_single_multiple: Topic = serde_json::from_str(
            r#"["0x0000000000000000000000000000000000000000000000000000000000000000"]"#,
        )
        .unwrap();
        let t_multiple_multiple_1: Topic = serde_json::from_str(
            r#"["0x0000000000000000000000000000000000000000000000000000000000000000",["0x0000000000000000000000000000000000000000000000000000000000000000"]]"#,
        )
        .unwrap();
        let t_multiple_multiple_2: Topic = serde_json::from_str(
            r#"[null,["0x0000000000000000000000000000000000000000000000000000000000000000"]]"#,
        )
        .unwrap();
        let t_null: Topic = serde_json::from_str(r#"null"#).unwrap();

        assert_eq!(
            t_single_single,
            Topic::Single(<VariadicValue<H256>>::Single(H256::zero()))
        );
        assert_eq!(
            t_single_multiple,
            Topic::Single(<VariadicValue<H256>>::Multiple(vec![H256::zero()]))
        );
        assert_eq!(
            t_multiple_multiple_1,
            Topic::Multiple(vec![
                <VariadicValue<H256>>::Single(H256::zero()),
                <VariadicValue<H256>>::Multiple(vec![H256::zero()])
            ])
        );
        assert_eq!(
            t_multiple_multiple_2,
            Topic::Multiple(vec![
                <VariadicValue<H256>>::Null,
                <VariadicValue<H256>>::Multiple(vec![H256::zero()])
            ])
        );
        assert_eq!(t_null, Topic::Single(<VariadicValue<H256>>::Null));
    }

    #[test]
    fn filter_changes_serialize() {
        let fc_log = FilterChanges::Logs(vec![Log::build()]);
        let fc_hashes = FilterChanges::Hashes(vec![H256::zero()]);
        let fc_empty = FilterChanges::Empty;

        assert_eq!(
            serde_json::to_string(&fc_log).unwrap(),
            r#"[{"removed":true,"logIndex":"0x1","transactionIndex":"0x1","transactionHash":"0x0000000000000000000000000000000000000000000000000000000000000000","blockHash":"0x0000000000000000000000000000000000000000000000000000000000000000","blockNumber":"0x1","address":"0x0000000000000000000000000000000000000000","data":"0x","topics":[]}]"#
        );
        assert_eq!(
            serde_json::to_string(&fc_hashes).unwrap(),
            r#"["0x0000000000000000000000000000000000000000000000000000000000000000"]"#
        );
        assert_eq!(serde_json::to_string(&fc_empty).unwrap(), r#"[]"#);
    }
}
