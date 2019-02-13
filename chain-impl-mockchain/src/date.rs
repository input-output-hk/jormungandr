use chain_core::property;

use std::{error, fmt, num::ParseIntError, str};

/// Non unique identifier of the transaction position in the
/// blockchain. There may be many transactions related to the same
/// `SlotId`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BlockDate {
    pub epoch: u64,
    pub slot_id: u64,
}

impl property::BlockDate for BlockDate {
    fn from_epoch_slot_id(epoch: u64, slot_id: u64) -> Self {
        BlockDate {
            epoch: epoch,
            slot_id: slot_id,
        }
    }
}

impl fmt::Display for BlockDate {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}.{}", self.epoch, self.slot_id)
    }
}

#[derive(Debug)]
pub enum BlockDateParseError {
    DotMissing,
    BadEpochId(ParseIntError),
    BadSlotId(ParseIntError),
}

const EXPECT_FORMAT_MESSAGE: &'static str = "expected block date format EPOCH.SLOT";

impl fmt::Display for BlockDateParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use BlockDateParseError::*;
        match self {
            DotMissing => write!(f, "{}", EXPECT_FORMAT_MESSAGE),
            BadEpochId(_) => write!(f, "invalid epoch ID, {}", EXPECT_FORMAT_MESSAGE),
            BadSlotId(_) => write!(f, "invalid slot ID, {}", EXPECT_FORMAT_MESSAGE),
        }
    }
}

impl error::Error for BlockDateParseError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        use BlockDateParseError::*;
        match self {
            DotMissing => None,
            BadEpochId(e) => Some(e),
            BadSlotId(e) => Some(e),
        }
    }
}

impl str::FromStr for BlockDate {
    type Err = BlockDateParseError;

    fn from_str(s: &str) -> Result<BlockDate, BlockDateParseError> {
        let (ep, sp) = match s.find('.') {
            None => return Err(BlockDateParseError::DotMissing),
            Some(pos) => (&s[..pos], &s[(pos + 1)..]),
        };
        let epoch = str::parse::<u64>(ep).map_err(BlockDateParseError::BadEpochId)?;
        let slot_id = str::parse::<u64>(sp).map_err(BlockDateParseError::BadSlotId)?;
        Ok(BlockDate { epoch, slot_id })
    }
}
