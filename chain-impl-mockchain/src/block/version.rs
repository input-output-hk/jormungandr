use num_derive::FromPrimitive;
use num_traits::FromPrimitive;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BlockVersion(pub(crate) u16);

impl BlockVersion {
    pub const fn new(v: u16) -> Self {
        BlockVersion(v)
    }

    pub const fn as_u16(&self) -> u16 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, FromPrimitive, PartialEq, Eq)]
pub enum BlockVersionTag {
    ConsensusNone = 0x0,
    ConsensusBft = 0x1,
    ConsensusGenesisPraos = 0x2,
}

impl BlockVersionTag {
    pub fn to_block_version(self) -> BlockVersion {
        BlockVersion::new(self as u16)
    }

    pub fn from_block_version(ver: BlockVersion) -> Option<BlockVersionTag> {
        BlockVersionTag::from_u16(ver.0)
    }
}
