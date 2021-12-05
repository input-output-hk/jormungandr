use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct VotePlanKey {
    pub alias: String,
    pub owner_alias: String,
}
