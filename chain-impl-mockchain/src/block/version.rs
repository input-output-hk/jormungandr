use strum_macros::{Display, EnumIter, EnumString, IntoStaticStr};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnyBlockVersion {
    Supported(BlockVersion),
    Unsupported(u16),
}

impl AnyBlockVersion {
    pub fn try_into_block_version(self) -> Option<BlockVersion> {
        match self {
            AnyBlockVersion::Supported(version) => Some(version),
            AnyBlockVersion::Unsupported(_) => None,
        }
    }
}

impl PartialEq<BlockVersion> for AnyBlockVersion {
    fn eq(&self, other: &BlockVersion) -> bool {
        match self {
            AnyBlockVersion::Supported(version) => version == other,
            AnyBlockVersion::Unsupported(_) => false,
        }
    }
}

impl From<u16> for AnyBlockVersion {
    fn from(n: u16) -> Self {
        match BlockVersion::from_u16(n) {
            Some(supported) => AnyBlockVersion::Supported(supported),
            None => AnyBlockVersion::Unsupported(n),
        }
    }
}

impl Into<u16> for AnyBlockVersion {
    fn into(self) -> u16 {
        match self {
            AnyBlockVersion::Supported(version) => version as u16,
            AnyBlockVersion::Unsupported(n) => n,
        }
    }
}

impl From<BlockVersion> for AnyBlockVersion {
    fn from(version: BlockVersion) -> Self {
        AnyBlockVersion::Supported(version)
    }
}

#[derive(Debug, Clone, Copy, EnumIter, PartialEq, Eq)]
pub enum BlockVersion {
    Genesis = 0,
    Ed25519Signed = 1,
    KesVrfproof = 2,
}

impl BlockVersion {
    pub fn from_u16(v: u16) -> Option<Self> {
        match v {
            0 => Some(BlockVersion::Genesis),
            1 => Some(BlockVersion::Ed25519Signed),
            2 => Some(BlockVersion::KesVrfproof),
            _ => None,
        }
    }

    pub fn get_consensus(self) -> Option<ConsensusVersion> {
        match self {
            BlockVersion::Genesis => None,
            BlockVersion::Ed25519Signed => Some(ConsensusVersion::Bft),
            BlockVersion::KesVrfproof => Some(ConsensusVersion::GenesisPraos),
        }
    }
}

#[derive(
    Debug, Clone, Copy, Display, EnumString, IntoStaticStr, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
pub enum ConsensusVersion {
    #[strum(to_string = "bft")]
    Bft = 1,
    #[strum(to_string = "genesis")]
    GenesisPraos = 2,
}

impl ConsensusVersion {
    pub fn from_u16(v: u16) -> Option<Self> {
        match v {
            1 => Some(ConsensusVersion::Bft),
            2 => Some(ConsensusVersion::GenesisPraos),
            _ => None,
        }
    }
    pub fn supported_block_versions(self) -> &'static [BlockVersion] {
        match self {
            ConsensusVersion::Bft => &[BlockVersion::Ed25519Signed],
            ConsensusVersion::GenesisPraos => &[BlockVersion::KesVrfproof],
        }
    }
}
