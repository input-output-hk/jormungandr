use crate::key::PublicKey;
use crate::transaction::Value;
use std::collections::{HashMap, HashSet};

// For each stake pool, the total stake value, and the value for the
// stake pool members.
pub type StakeDistribution = HashMap<PublicKey, (Value, HashMap<PublicKey, Value>)>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StakeKeyInfo {
    /// Current stake pool this key is a member of, if any.
    pub pool: Option<PublicKey>,
    // - reward account
    // - registration deposit (if variable)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StakePoolInfo {
    //owners: HashSet<PublicKey>,
    pub members: HashSet<PublicKey>,
}
