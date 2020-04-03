use crate::blockchain;
use chain_impl_mockchain::certificate::PoolId;
use chain_impl_mockchain::stake::PoolStakeInformation;
use std::cmp::Ordering;
use std::sync::Arc;

/// In order to send the sea in the bottle message we need to select the best fit stake pools
/// in this case we can retrieve the PoolId sorted from a `blockchain::Ref`
pub fn stake_pools_sorted_by<F>(
    block_ref: Arc<blockchain::Ref>,
    comp: F,
) -> impl IntoIterator<Item = PoolId>
where
    F: Fn(&PoolStakeInformation, &PoolStakeInformation) -> Ordering,
{
    match block_ref.epoch_leadership_schedule().stake_distribution() {
        None => Vec::new(),
        Some(distribution) => {
            let mut pools: Vec<(&PoolId, &PoolStakeInformation)> =
                distribution.to_pools.iter().collect();
            pools.sort_by(|&(_, information_a), &(_, information_b)| {
                comp(information_a, information_b)
            });
            pools.iter().map(|&(id, _)| id.clone()).rev().collect()
        }
    }
}

/// Simple cmp function for `PoolStakeInformation` by `stake.total` attribute.
/// Used together with `stake_pools_sorted_by`
/// ```ignore
/// stake_pools_sorted_by(block_ref, cmp_pool_stake_information_by_stake_total)
/// ```
pub fn cmp_pool_stake_information_by_stake_total(
    pool_a: &PoolStakeInformation,
    pool_b: &PoolStakeInformation,
) -> Ordering {
    pool_a.stake.total.cmp(&pool_b.stake.total)
}
