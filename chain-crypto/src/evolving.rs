use crate::key::{AsymmetricKey, SecretKey};

/// Evolving status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvolvingStatus {
    Success,
    Failed,
}

pub trait KeyEvolvingAlgorithm: AsymmetricKey {
    /// Get the period associated with this signature
    fn get_period(key: &Self::Secret) -> u32;

    /// Update the secret key to the next period
    ///
    /// if EvolvingStatus::Failed is returned, then the key couldn't be updated
    fn update(key: &mut Self::Secret) -> EvolvingStatus;
}

impl<A: KeyEvolvingAlgorithm> SecretKey<A> {
    /// Evolve the secret key to the next period
    pub fn evolve(key: &mut Self) -> EvolvingStatus {
        A::update(&mut key.0)
    }
    /// Get the period associated with the current instance of the key
    pub fn get_period(key: &Self) -> u32 {
        A::get_period(&key.0)
    }
}
