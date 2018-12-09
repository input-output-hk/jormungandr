mod chain;
mod process;

pub use self::chain::{Blockchain, BlockchainR};
pub use self::process::process;
use cbor_event::{de::RawCbor};

pub type Hash = cardano::hash::Blake2b256;

pub trait Block : Clone {
    fn get_hash(&self) -> Hash;

    fn get_parent(&self) -> Hash;

    fn serialize(&self) -> Vec<u8>;
    fn deserialize(bytes: &[u8]) -> Self;
}

impl Block for cardano::block::Block {
    fn get_hash(&self) -> Hash {
        (*self.get_header().compute_hash()).into()
    }

    fn get_parent(&self) -> Hash {
        (*self.get_header().get_previous_header()).into()
    }

    fn serialize(&self) -> Vec<u8> {
        cbor!(self).unwrap()
    }

    fn deserialize(bytes: &[u8]) -> Self {
        RawCbor::from(bytes).deserialize_complete().unwrap()
    }
}
