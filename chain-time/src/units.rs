use std::time::Duration;

/// Represent a Duration where the maximum precision is in the second
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DurationSeconds(pub u64);

impl From<u64> for DurationSeconds {
    fn from(v: u64) -> Self {
        Self(v)
    }
}

impl From<DurationSeconds> for u64 {
    fn from(v: DurationSeconds) -> Self {
        v.0
    }
}

impl From<DurationSeconds> for Duration {
    fn from(v: DurationSeconds) -> Self {
        Duration::from_secs(v.0)
    }
}
