#[derive(Debug, PartialEq, Eq)]
pub enum DerivationType {
    Soft(u32),
    Hard(u32),
}

pub type DerivationIndex = u32;

impl DerivationType {
    pub fn from_index(index: DerivationIndex) -> Self {
        if index >= 0x80000000 {
            DerivationType::Hard(index)
        } else {
            DerivationType::Soft(index)
        }
    }
}

/// Ed25519-bip32 Scheme Derivation version
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DerivationScheme {
    V1,
    V2,
}

impl Default for DerivationScheme {
    fn default() -> Self {
        DerivationScheme::V2
    }
}
