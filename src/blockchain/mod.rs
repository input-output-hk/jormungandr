mod chain;
mod process;

pub use self::chain::{Blockchain, BlockchainR};
pub use self::process::process;
use cbor_event::{de::RawCbor};

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

impl Date for cardano::block::date::BlockDate {
    fn serialize(&self) -> u64 {
        match self {
            cardano::block::BlockDate::Boundary(epoch) => epoch << 16,
            cardano::block::BlockDate::Normal(s) => { assert!(s.slotid < 65535); ((s.epoch as u64) << 16) | ((s.slotid + 1) as u64) }
        }
    }

    fn deserialize(n: u64) -> Self {
        let epoch = n >> 16;
        let slot = n & 65535;
        if slot == 0 {
            cardano::block::BlockDate::Boundary(epoch)
        } else {
            cardano::block::BlockDate::Normal(
                cardano::block::EpochSlotId { epoch, slotid: (slot - 1) as u16 })
        }
    }
}

impl Block for cardano::block::Block {
    fn get_hash(&self) -> Hash {
        (*self.get_header().compute_hash()).into()
    }

    fn get_parent(&self) -> Hash {
        (*self.get_header().get_previous_header()).into()
    }

    type Date = cardano::block::date::BlockDate;

    fn get_date(&self) -> Self::Date {
        self.get_header().get_blockdate()
    }

    fn serialize(&self) -> Vec<u8> {
        cbor!(self).unwrap()
    }

    fn deserialize(bytes: &[u8]) -> Self {
        RawCbor::from(bytes).deserialize_complete().unwrap()
    }
}
