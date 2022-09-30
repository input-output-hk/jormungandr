use rand_chacha::ChaChaRng;
use rand_core::{RngCore, SeedableRng};
use std::{
    fmt::{self, Display, Formatter},
    str::FromStr,
};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Seed([u8; 32]);

impl Display for Seed {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        hex::encode(self.0).fmt(f)
    }
}

impl FromStr for Seed {
    type Err = hex::FromHexError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes = hex::decode(s)?;

        let mut seed = Seed::zero();

        if bytes.len() != seed.0.len() {
            Err(hex::FromHexError::InvalidStringLength)
        } else {
            seed.0.copy_from_slice(&bytes);

            Ok(seed)
        }
    }
}

#[derive(Clone)]
pub struct Random<RNG: RngCore + Sized> {
    rng: RNG,
    seed: Seed,
}

impl Seed {
    fn zero() -> Self {
        Seed([0; 32])
    }

    pub fn generate<RNG: RngCore>(mut rng: RNG) -> Self {
        let mut seed = Seed::zero();
        rng.fill_bytes(&mut seed.0);
        seed
    }
}

impl Random<ChaChaRng> {
    pub fn new(seed: Seed) -> Self {
        let rng = ChaChaRng::from_seed(seed.0);
        Self { rng, seed }
    }

    /// derive the Context into a new context, seeding a new RNG from the original
    /// Context (so reproducibility is still available).
    pub fn derive(&mut self) -> Self {
        let seed = Seed::generate(self.rng_mut());
        let rng = ChaChaRng::from_seed(seed.0);
        Self { rng, seed }
    }
}

impl<RNG: RngCore> Random<RNG> {
    pub fn rng_mut(&mut self) -> &mut RNG {
        &mut self.rng
    }

    /// retrieve the original seed of the pseudo random generator
    #[inline]
    pub fn seed(&self) -> &Seed {
        &self.seed
    }
}
