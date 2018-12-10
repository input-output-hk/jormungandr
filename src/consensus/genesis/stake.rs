use std::ops::{Add,Sub};
use std::collections::BTreeMap;
use std::collections::btree_map::Entry;

// TODO: PublicKey
use super::super::super::secure::crypto::{vrf, kes, sign};
use super::super::super::secure::crypto::sign::SignatureAlgorithm;
use super::super::super::secure::crypto::kes::KES;

use super::identity::StakerIdentity;

/// Units of stake
///
/// This should always be <= to StakeTotal
#[derive(Clone,Copy,PartialEq,Eq,PartialOrd,Ord)]
pub struct StakeUnits(u128);

impl Add<StakeUnits> for StakeUnits {
    type Output = StakeUnits;
    fn add(self, rhs: StakeUnits) -> Self {
        StakeUnits(self.0 + rhs.0)
    }
}
impl Sub<StakeUnits> for StakeUnits {
    type Output = StakeUnits;
    fn sub(self, rhs: StakeUnits) -> Self {
        StakeUnits(self.0 - rhs.0)
    }
}


/// Total amount of unit of stake in the system
#[derive(Clone,Copy,PartialEq,Eq,PartialOrd,Ord)]
pub struct StakeTotal(u128);

impl Add<StakeUnits> for StakeTotal {
    type Output = StakeTotal;
    fn add(self, rhs: StakeUnits) -> Self {
        StakeTotal(self.0 + rhs.0)
    }
}
impl Sub<StakeUnits> for StakeTotal {
    type Output = StakeTotal;
    fn sub(self, rhs: StakeUnits) -> Self {
        StakeTotal(self.0 - rhs.0)
    }
}

/// Percent Stake in the system between 0% (0.0) and 100% (1.0)
///
/// * 0.0: no stake in the system
/// * 1.0: full stake in the system
#[derive(Clone,Copy,PartialEq,PartialOrd)]
pub struct PercentStake(pub f64);

impl StakeTotal {
    pub fn percent(&self, units: StakeUnits) -> PercentStake {
        assert!(units.0 <= self.0);
        PercentStake((units.0 as f64) / (self.0 as f64))
    }
}

pub struct StakerPublicInformation {
    vrf_key: vrf::PublicKey,
    block_key: Option<<sign::Ed25519 as SignatureAlgorithm>::PublicKey>,
}

/// Distribution of stake according to identities
#[derive(Clone)]
pub struct StakeDistribution {
    total: StakeTotal,
    map: BTreeMap<StakerIdentity, StakeUnits>,
}

impl StakeDistribution {
    pub fn create() -> Self {
        StakeDistribution { total: StakeTotal(0), map: BTreeMap::new() }
    }

    pub fn add(&mut self, id: StakerIdentity, units: StakeUnits) {
        self.map.entry(id).and_modify(|v| *v = *v + units).or_insert(units);
        self.total = self.total + units;
    }

    pub fn remove(&mut self, id: StakerIdentity, units: StakeUnits) {
        match self.map.entry(id) {
            Entry::Vacant(_) => {
                // FIXME don't do anything for now, but it should likely be reported back.
            },
            Entry::Occupied(mut entry) => {
                let mut e = entry.get_mut();
                *e = *e - units;
                self.total = self.total - units;
            },
        }
    }
}

/// Delegation of stake from one staker to another
pub struct StakeDelegation {
    map: BTreeMap<StakerIdentity, StakerIdentity>,
}
