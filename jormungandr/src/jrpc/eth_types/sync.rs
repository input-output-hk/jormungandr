use super::number::Number;
use serde::{Serialize, Serializer};

/// Sync info
#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SyncInfo {
    /// Starting block
    starting_block: Number,
    /// Current block
    current_block: Number,
    /// Highest block seen so far
    highest_block: Number,
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
    pub fn build(starting_block: u64, current_block: u64, highest_block: u64) -> Self {
        Self::Info(SyncInfo {
            starting_block: starting_block.into(),
            current_block: current_block.into(),
            highest_block: highest_block.into(),
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
            starting_block: 0.into(),
            current_block: 0.into(),
            highest_block: 0.into(),
        });

        assert_eq!(serde_json::to_string(&ss_none).unwrap(), "false");
        assert_eq!(
            serde_json::to_string(&ss_info).unwrap(),
            r#"{"startingBlock":"0x0","currentBlock":"0x0","highestBlock":"0x0"}"#
        );
    }
}
