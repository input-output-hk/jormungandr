extern crate serde_derive;
use self::serde_derive::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct AccountState {
    pub value: i32,
    pub counter: i32,
}
