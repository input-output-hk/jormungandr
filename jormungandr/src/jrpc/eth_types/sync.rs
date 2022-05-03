use chain_evm::ethereum_types::U256;
use serde::{Serialize, Serializer};

/// Sync info
#[derive(Default, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SyncInfo {
    /// Starting block
    pub starting_block: U256,
    /// Current block
    pub current_block: U256,
    /// Highest block seen so far
    pub highest_block: U256,
}

/// Sync status
#[derive(Debug, PartialEq, Eq)]
pub enum SyncStatus {
    /// Info when syncing
    Info(SyncInfo),
    /// Not syncing
    #[allow(dead_code)]
    None,
}

impl SyncStatus {
    pub fn build() -> Self {
        Self::Info(SyncInfo {
            starting_block: U256::zero(),
            current_block: U256::zero(),
            highest_block: U256::zero(),
        })
    }
}

impl Serialize for SyncStatus {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match *self {
            SyncStatus::Info(ref info) => info.serialize(serializer),
            SyncStatus::None => false.serialize(serializer),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sync_status_json_deserialize() {
        let ss_none = SyncStatus::None;
        let ss_info = SyncStatus::Info(SyncInfo {
            starting_block: U256::zero(),
            current_block: U256::zero(),
            highest_block: U256::zero(),
        });

        assert_eq!(serde_json::to_string(&ss_none).unwrap(), "false");
        assert_eq!(serde_json::to_string(&ss_info).unwrap(), "{\"startingBlock\":\"0x0\",\"currentBlock\":\"0x0\",\"highestBlock\":\"0x0\"}");
    }
}
