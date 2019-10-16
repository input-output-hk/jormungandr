use super::cstruct;
use std::num::NonZeroUsize;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockVersion {
    Genesis,
    Ed25519Signed,
    KesVrfproof,
}

impl BlockVersion {
    pub fn from_u16(v: u16) -> Option<Self> {
        match v {
            cstruct::VERSION_UNSIGNED => Some(BlockVersion::Genesis),
            cstruct::VERSION_BFT => Some(BlockVersion::Ed25519Signed),
            cstruct::VERSION_GP => Some(BlockVersion::KesVrfproof),
            _ => None,
        }
    }

    pub fn to_u16(self) -> u16 {
        match self {
            BlockVersion::Genesis => cstruct::VERSION_UNSIGNED,
            BlockVersion::Ed25519Signed => cstruct::VERSION_BFT,
            BlockVersion::KesVrfproof => cstruct::VERSION_GP,
        }
    }

    pub const fn get_size(self) -> NonZeroUsize {
        const SIZE: [NonZeroUsize; 3] = [
            unsafe { NonZeroUsize::new_unchecked(cstruct::HEADER_COMMON_SIZE) },
            unsafe { NonZeroUsize::new_unchecked(cstruct::HEADER_BFT_SIZE) },
            unsafe { NonZeroUsize::new_unchecked(cstruct::HEADER_GP_SIZE) },
        ];
        SIZE[self as usize]
    }

    pub const fn get_auth_size(self) -> NonZeroUsize {
        const SIZE: [NonZeroUsize; 3] = [
            unsafe { NonZeroUsize::new_unchecked(cstruct::HEADER_COMMON_SIZE) },
            unsafe { NonZeroUsize::new_unchecked(cstruct::HEADER_BFT_AUTHED_SIZE) },
            unsafe { NonZeroUsize::new_unchecked(cstruct::HEADER_GP_AUTHED_SIZE) },
        ];
        SIZE[self as usize]
    }
}
