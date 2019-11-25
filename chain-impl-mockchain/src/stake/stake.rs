use crate::value::Value;
use std::ops::{Add, AddAssign};

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Stake(u64);

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct StakeUnit(Stake);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PercentStake {
    pub stake: Stake,
    pub total: Stake,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct SplitValueIn {
    pub parts: StakeUnit,
    pub remaining: Stake,
}

impl Stake {
    pub fn from_value(v: Value) -> Self {
        Stake(v.0)
    }

    pub fn zero() -> Self {
        Stake(0)
    }

    pub fn sum<I>(values: I) -> Self
    where
        I: Iterator<Item = Self>,
    {
        values.fold(Stake(0), |acc, v| acc + v)
    }

    /// Divide a value by n equals parts, with a potential remainder
    pub fn split_in(self, n: u32) -> SplitValueIn {
        let n = n as u64;
        SplitValueIn {
            parts: StakeUnit(Stake(self.0 / n)),
            remaining: Stake(self.0 % n),
        }
    }
}

impl StakeUnit {
    pub fn scale(&self, n: u32) -> Stake {
        Stake((self.0).0.checked_mul(n as u64).unwrap())
    }
}

impl PercentStake {
    pub fn new(stake: Stake, total: Stake) -> Self {
        assert!(stake <= total);
        PercentStake { stake, total }
    }

    pub fn as_float(&self) -> f64 {
        (self.stake.0 as f64) / (self.total.0 as f64)
    }
}

impl Add for Stake {
    type Output = Stake;

    fn add(self, other: Self) -> Self {
        Stake(self.0 + other.0)
    }
}

impl AddAssign for Stake {
    fn add_assign(&mut self, other: Self) {
        *self = Self(self.0 + other.0)
    }
}

impl std::fmt::Display for Stake {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
