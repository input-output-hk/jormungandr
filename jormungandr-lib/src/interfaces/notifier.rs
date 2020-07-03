use crate::crypto::hash::Hash;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum JsonMessage {
    NewBlock(Hash),
    NewTip(Hash),
}

impl Into<String> for JsonMessage {
    fn into(self) -> String {
        serde_json::to_string(&self).unwrap()
    }
}
