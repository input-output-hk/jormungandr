use crate::{crypto::hash::Hash, interfaces::stake::Stake};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StakeDistributionDto {
    pub epoch: u32,
    pub stake: StakeDistribution,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StakeDistribution {
    pub dangling: Stake,
    pub unassigned: Stake,
    pub pools: Vec<(Hash, Stake)>,
}
