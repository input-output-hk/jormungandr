use rand::RngCore;
use std::{
    fmt::{self, Debug, Display},
    str::FromStr,
};

/// unique identifier for an identity in with the KMS
#[derive(Ord, PartialOrd, Eq, PartialEq, Clone, Copy, Hash)]
pub struct Id {
    raw: [u8; 32],
}

impl Id {
    /// create a new ID with the given raw bytes
    pub const fn new(raw: [u8; 32]) -> Self {
        Self { raw }
    }

    const fn zero() -> Self {
        Self::new([0; 32])
    }

    /// generate a random Id with the `thread_rng`
    pub fn generate() -> Self {
        let mut rng = rand::thread_rng();
        Self::generate_with(&mut rng)
    }

    /// generate a Random Id with the given Random number generator
    pub fn generate_with<RNG>(rng: &mut RNG) -> Self
    where
        RNG: RngCore,
    {
        let mut id = Id::zero();
        rng.fill_bytes(&mut id.raw);
        id
    }
}

impl From<[u8; 32]> for Id {
    fn from(raw: [u8; 32]) -> Self {
        Self::new(raw)
    }
}

impl Into<[u8; 32]> for Id {
    fn into(self) -> [u8; 32] {
        self.raw
    }
}

impl Debug for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Id")
            .field("raw", &hex::encode(&self.raw))
            .finish()
    }
}

impl Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(&hex::encode(&self.raw), f)
    }
}

impl FromStr for Id {
    type Err = hex::FromHexError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut id = Id::zero();
        hex::decode_to_slice(s, &mut id.raw)?;
        Ok(id)
    }
}
