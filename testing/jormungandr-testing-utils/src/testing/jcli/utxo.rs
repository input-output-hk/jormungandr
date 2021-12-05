extern crate serde_derive;
use self::serde_derive::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Utxo {
    pub in_idx: i32,
    pub in_txid: String,
    pub out_addr: String,
    pub out_value: i32,
}
