//! define the Blockchain settings
//!

use crate::key::Hash;
use crate::update::{SettingsDiff, ValueDiff};
use chain_core::property;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Version {
    major: u16,
    minor: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Settings {
    pub last_block_id: Hash,
}

#[derive(Debug)]
pub enum Error {
    InvalidCurrentBlockId(Hash, Hash),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::InvalidCurrentBlockId(current_one, update_one) => {
                write!(f, "Cannot apply Setting Update. Update needs to be applied to from block {:?} but received {:?}", update_one, current_one)
            }
        }
    }
}
impl std::error::Error for Error {}

impl property::Settings for Settings {
    type Update = SettingsDiff;
    type Error = Error;
    type Block = crate::block::SignedBlock;

    fn diff(&self, input: &Self::Block) -> Result<Self::Update, Self::Error> {
        use chain_core::property::Block;

        let mut update = <Self::Update as property::Update>::empty();

        update.block_id = ValueDiff::Replace(self.last_block_id.clone(), input.id());

        Ok(update)
    }
    fn apply(&mut self, update: Self::Update) -> Result<(), Self::Error> {
        match update.block_id {
            ValueDiff::None => {}
            ValueDiff::Replace(expected_current_block_id, new_block_id) => {
                if expected_current_block_id != self.last_block_id {
                    return Err(Error::InvalidCurrentBlockId(
                        self.last_block_id,
                        expected_current_block_id,
                    ));
                } else {
                    self.last_block_id = new_block_id;
                }
            }
        }
        Ok(())
    }

    fn tip(&self) -> <Self::Block as property::Block>::Id {
        self.last_block_id.clone()
    }
}
