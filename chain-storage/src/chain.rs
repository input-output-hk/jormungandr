use chain_core::property::Block;
use super::error::Error;

pub trait ChainState: std::marker::Sized + Clone + Eq {
    type Block: Block;
    type GenesisData;
    type Delta: ChainStateDelta;

    fn new(genesis_data: &Self::GenesisData) -> Result<Self, Error>;

    fn apply_block(&mut self, block: &Self::Block) -> Result<(), Error>;

    fn get_last_block(&self) -> <Self::Block as Block>::Id;

    fn get_chain_length(&self) -> u64;

    fn diff(from: &Self, to: &Self) -> Result<Self::Delta, Error>;

    fn apply_delta(&mut self, delta: Self::Delta) -> Result<(), Error>;
}

pub trait ChainStateDelta {
    //fn merge(a: &Self, b: &Self) -> Self;

    fn serialize(&self) -> Vec<u8>;
    fn deserialize(bytes: &[u8]) -> Self;
}
