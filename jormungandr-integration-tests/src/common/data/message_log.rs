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
        if self.is_status_a_string() {
            return false;
        }
        match self.status.get("InABlock") {
            Some(_) => true,
            None => false,
        }
    }

    pub fn is_rejected(&self) -> bool {
        if self.is_status_a_string() {
            return false;
        }
        match self.status.get("Rejected") {
            Some(_) => true,
            None => false,
        }
    }

    pub fn get_reject_message(&self) -> String {
        if !self.is_rejected() {
            panic!("Cannot get rejected message from non rejected fragment");
        }
        match self.status.get("Rejected").unwrap().get("reason") {
            Some(x) => x.as_str().unwrap().to_string(),
            None => panic!("empty reasn"),
        }
    }

    pub fn is_pending(&self) -> bool {
        if !self.is_status_a_string() {
            return false;
        }
        self.status.as_str().unwrap() == "Pending"
    }

    fn is_status_a_string(&self) -> bool {
        self.status.is_string()
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
