mod chain;
mod process;
mod cardano_classic;

pub use self::chain::{Blockchain, BlockchainR};
pub use self::process::process;

// FIXME: abstract over Hash.
pub type Hash = cardano::hash::Blake2b256;

/// A trait representing block dates. Dates can be compared, ordered
/// and serialized as integers.
pub trait Date : Eq + Ord + Clone {
    fn serialize(&self) -> u64;
    fn deserialize(n: u64) -> Self;
}

/// A trait representing blocks. Blocks have a unique identifier
/// (`Hash`), a link to the previous block (the parent), and a
/// date. They can be serialized as a sequence of bytes.
pub trait Block : Clone {

    fn get_hash(&self) -> Hash;

    fn get_parent(&self) -> Hash;

    type Date: Date;

    fn get_date(&self) -> Self::Date;

    fn serialize(&self) -> Vec<u8>;
    fn deserialize(bytes: &[u8]) -> Self;
}

pub trait ChainState: std::marker::Sized + Clone {
    type Block: Block;
    type Error: std::error::Error; // FIXME: introduce local error type
    type GenesisData;

    fn new(genesis_data: &Self::GenesisData) -> Result<Self, Self::Error>;

    fn apply_block(&mut self, block: &Self::Block) -> Result<(), Self::Error>;

    fn get_last_block(&self) -> Hash;

    fn get_chain_length(&self) -> u64;
}
