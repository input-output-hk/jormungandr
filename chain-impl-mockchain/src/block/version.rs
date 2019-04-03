use lazy_static::lazy_static;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use std::collections::BTreeMap;
use strum::IntoEnumIterator;
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

#[derive(Debug, Clone, Copy, EnumIter, FromPrimitive, PartialEq, Eq)]
pub enum BlockVersion {
    Genesis = 0,
    Ed25519Signed = 1,
    KesVrfproof = 2,
}

impl BlockVersion {
    pub fn get_consensus(self) -> ConsensusVersion {
        match self {
            BlockVersion::Genesis => ConsensusVersion::None,
            BlockVersion::Ed25519Signed => ConsensusVersion::Bft,
            BlockVersion::KesVrfproof => ConsensusVersion::GenesisPraos,
        }
    }
}

#[derive(
    Debug,
    Clone,
    Copy,
    Display,
    EnumString,
    FromPrimitive,
    IntoStaticStr,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
)]
pub enum ConsensusVersion {
    #[strum(to_string = "none")]
    None = 0,
    #[strum(to_string = "bft")]
    Bft = 1,
    #[strum(to_string = "genesis")]
    GenesisPraos = 2,
}

impl ConsensusVersion {
    pub fn supported_block_versions(self) -> &'static [BlockVersion] {
        lazy_static! {
            static ref MAPPING: BTreeMap<u16, Vec<BlockVersion>> = {
                let mut map = BTreeMap::<_, Vec<_>>::new();
                for block_version in BlockVersion::iter() {
                    map.entry(block_version.get_consensus() as u16)
                        .or_default()
                        .push(block_version)
                }
                map
            };
        }
        MAPPING
            .get(&(self as u16))
            .map(AsRef::as_ref)
            .unwrap_or_default()
    }
}
