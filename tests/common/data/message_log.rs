extern crate serde_derive;
use self::serde_derive::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Fragment {
    pub fragment_id: String,
    pub last_updated_at: String,
    pub recieved_at: Option<String>,
    pub status: serde_yaml::Value,
}

impl Fragment {
    pub fn is_in_block(&self) -> bool {
        if self.status.is_string() {
            return false;
        }
        match self.status.get("InABlock") {
            Some(_) => true,
            None => false,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Status {
    pub value: Option<String>,
    pub in_a_block: Option<InABlock>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InABlock {
    pub epoch_slot: String,
}
