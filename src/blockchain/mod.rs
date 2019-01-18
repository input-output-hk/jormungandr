mod chain;
mod process;

pub use self::chain::{Blockchain, BlockchainR};
pub use self::process::process;

pub type Hash = cardano::hash::Blake2b256;

pub trait Block: Clone {
    fn get_hash(&self) -> Hash;

    fn get_parent(&self) -> Hash;

    fn as_bytes(&self) -> Vec<u8>;
}

impl Block for cardano::block::Block {
    fn get_hash(&self) -> Hash {
        unimplemented!()
        // self.header().compute_hash()
    }

    fn get_parent(&self) -> Hash {
        unimplemented!()
        // self.header().get_previous_header()
    }

    fn as_bytes(&self) -> Vec<u8> {
        cbor!(self).unwrap()
    }
}
