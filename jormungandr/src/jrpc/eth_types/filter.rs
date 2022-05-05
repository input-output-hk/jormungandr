use super::block_number::BlockNumber;

/// Filter
#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct Filter {
    /// From Block
    pub from_block: Option<BlockNumber>,
    /// To Block
    pub to_block: Option<BlockNumber>,
    // TODO implement
    // /// Address
    // pub address: Option<FilterAddress>,
    // /// Topics
    // pub topics: Option<Topic>,
}
