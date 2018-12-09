use std::collections::BTreeMap;

// TODO: PublicKey
use super::super::super::secure::crypto::{vrf, kes, sign};
use super::super::super::secure::crypto::sign::SignatureAlgorithm;
use super::super::super::secure::crypto::kes::KES;

use super::identity::StakerIdentity;

/// Units of stake
///
/// This should always be <= to StakeTotal
pub struct StakeUnits(u128);

/// Total amount of unit of stake in the system
pub struct StakeTotal(u128);

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

/// Distribution
pub struct StakeDistribution {
    map: BTreeMap<StakerIdentity, StakeUnits>,
}

/// Delegation of stake from one staker to another
pub struct StakeDelegation {
    map: BTreeMap<StakerIdentity, StakerIdentity>,
}
