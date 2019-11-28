use crate::jcli_app::certificate::Error;
use chain_crypto::Blake2b256;
use chain_impl_mockchain::account::{DelegationRatio, DelegationType};
use chain_impl_mockchain::accounting::account::DELEGATION_RATIO_MAX_DECLS;
use std::convert::TryFrom;
use std::error::Error as StdError;
use std::str::FromStr;
use structopt::StructOpt;

#[derive(StructOpt)]
pub struct WeightedPoolIds {
    /// hex-encoded stake pool IDs and their numeric weights in format "pool_id:weight".
    /// If weight is not provided, it defaults to 1.
    #[structopt(name = "STAKE_POOL_IDS", raw(required = "true"))]
    pool_ids: Vec<WeightedPoolId>,
}

struct WeightedPoolId {
    pool_id: Blake2b256,
    weight: u8,
}

impl<'a> TryFrom<&'a WeightedPoolIds> for DelegationType {
    type Error = Error;

    fn try_from(pool_ids: &'a WeightedPoolIds) -> Result<Self, Self::Error> {
        let pool_ids = &pool_ids.pool_ids;
        let delegation = match pool_ids.len() {
            1 => DelegationType::Full(pool_ids[0].pool_id.into()),
            _ => DelegationType::Ratio(delegation_ratio(pool_ids)?),
        };
        Ok(delegation)
    }
}

fn delegation_ratio(pool_ids: &[WeightedPoolId]) -> Result<DelegationRatio, Error> {
    if pool_ids.len() > DELEGATION_RATIO_MAX_DECLS {
        return Err(Error::TooManyPoolDelegations {
            actual: pool_ids.len(),
            max: DELEGATION_RATIO_MAX_DECLS,
        });
    }
    let parts = delegation_ratio_sum(pool_ids)?;
    let pools = pool_ids
        .iter()
        .map(|pool_id| (pool_id.pool_id.into(), pool_id.weight))
        .collect();
    DelegationRatio::new(parts, pools).ok_or_else(|| Error::InvalidPoolDelegation)
}

fn delegation_ratio_sum(pool_ids: &[WeightedPoolId]) -> Result<u8, Error> {
    let parts = pool_ids
        .iter()
        .map(|pool_id| match pool_id.weight {
            0 => Err(Error::PoolDelegationWithZeroWeight),
            weight => Ok(weight as u64),
        })
        .sum::<Result<_, _>>()?;
    u8::try_from(parts).map_err(|_| Error::InvalidPoolDelegationWeights {
        actual: parts,
        max: u8::max_value() as u64,
    })
}

impl FromStr for WeightedPoolId {
    type Err = Box<dyn StdError>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut split = s.splitn(2, ':');
        Ok(WeightedPoolId {
            pool_id: split.next().unwrap().parse()?,
            weight: split.next().map_or(Ok(1), str::parse)?,
        })
    }
}
