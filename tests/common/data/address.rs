#[derive(Debug)]
pub struct Utxo {
    pub private_key: String,
    pub public_key: String,
    pub address: String,
}
#[derive(Debug)]
pub struct Account {
    pub private_key: String,
    pub public_key: String,
    pub address: String,
}
#[derive(Debug)]
pub struct Delegation {
    pub private_key: String,
    pub public_key: String,
    pub address: String,
    pub delegation_address: String,
}
