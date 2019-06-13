extern crate serde_derive;
use self::serde_derive::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct AccountState {
    pub value: u32,
    pub counter: u32,
}
