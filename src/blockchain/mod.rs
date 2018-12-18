mod chain;
mod process;
mod cardano_classic;

pub use self::chain::{Blockchain, BlockchainR};
pub use self::process::process;

pub type Hash = cardano::hash::Blake2b256;

pub trait Date : Eq + Ord + Clone {
    fn serialize(&self) -> u64;
    fn deserialize(n: u64) -> Self;
}

pub trait Block : Clone {

    fn get_hash(&self) -> Hash;

    fn get_parent(&self) -> Hash;

    type Date: Date;

    fn get_date(&self) -> Self::Date;

    fn serialize(&self) -> Vec<u8>;
    fn deserialize(bytes: &[u8]) -> Self;
}
