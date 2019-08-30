use crate::value::Value;
use crate::block::Epoch;

#[derive(Debug,Clone,Copy)]
pub enum ReducingType {
    Linear,
    Halvening,
}

#[derive(Debug,Clone,Copy)]
pub struct Ratio {
    pub numerator: u64,
    pub denominator: u64,
}

#[derive(Debug, Clone, Copy)]
pub enum TaxType {
    Fixed(Value),
    RatioLimit(Ratio, Option<u64>),
}

/// Parameters for rewards calculation. This controls:
/// 
/// * Rewards contributions
/// * Treasury cuts
#[derive(Debug, Clone)]
pub struct Parameters {
    /// Tax cut of the treasury which is applied straight after the reward pot 
    /// is fully known
    treasury_tax: TaxType,

    //pool_owners_tax: TaxType,

    /// This is an initial_value for the linear or halvening function.
    /// In the case of the linear function it is the value that is going to be calculated
    /// from the contribution.
    rewards_initial_value: u64,
    /// This is the ratio used by either the linear or the halvening function.
    /// The semantic and result of this is deeply linked to either.
    rewards_reducement_ratio: Ratio,
    /// The type of reduction
    reducing_type: ReducingType,
    /// Number of epoch between reduction phase, cannot be 0
    reducing_epoch_rate: u64,
}

/// The reward to distribute to treasury and pools
#[derive(Debug, Clone)]
pub struct Distribution {
    pub treasury: Value,
    pub pools: Value,
}

/// Calculate the reward contribution from the parameters
///
/// Note that the contribution in the system is still bounded by the remaining
/// rewards pot, which is not taken in considering for this calculation.
pub fn rewards_contribution_calculation(epoch: Epoch, params: &Parameters) -> Value {
    assert!(params.reducing_epoch_rate != 0);
    let zone = epoch as u64 / params.reducing_epoch_rate;
    match params.reducing_type {
        ReducingType::Linear => {
            // C - rratio * (#epoch / erate)
            let rr = &params.rewards_reducement_ratio;
            let reduce_by = (rr.numerator * zone) / rr.denominator;
            if params.rewards_initial_value >= reduce_by {
                Value(params.rewards_initial_value - reduce_by)
            } else {
                Value(params.rewards_initial_value)
            }
        }
        ReducingType::Halvening => {
            // mathematical formula is : C * rratio ^ (#epoch / erate)
            // although we perform it as a for loop, with the rationale
            // that it allow for integer computation and that the reduce_epoch_rate
            // should prevent growth to large amount of zones
            let rr = &params.rewards_reducement_ratio;
            const SCALE : u128 = 10^18;

            let mut acc = params.rewards_initial_value as u128 * SCALE;
            for _ in 0..zone {
                acc *= rr.numerator as u128;
                acc /= rr.denominator as u128;
            }

            Value((acc / SCALE) as u64)
        }
    }
}

/// Distribute a pot of value to treasury and pools according to redistribution parameters
pub fn distribute(v: Value, params: &Parameters) -> Distribution {
    match params.treasury_tax {
        TaxType::Fixed(fixed) => {
            if v > fixed {
                Distribution {
                    treasury: fixed,
                    // treasury_cut is < to v so it's safe to unwrap
                    pools: (v - fixed).unwrap(),
                }
            } else {
                Distribution {
                    treasury: v,
                    pools: Value::zero(),
                }
            }
        }
        TaxType::RatioLimit(rr, olimit) => {
            const SCALE : u128 = 10^9;
            let out = ((((v.0 as u128 * SCALE) * rr.numerator as u128) / rr.denominator as u128) / SCALE) as u64;
            let treasury_cut = match olimit {
                None => Value(out),
                Some(limit) => Value(std::cmp::min(limit, out)),
            };
            if v > treasury_cut {
                Distribution {
                    treasury: treasury_cut,
                    // treasury_cut is < to v so it's safe to unwrap
                    pools: (v - treasury_cut).unwrap(),
                }
            } else {
                Distribution {
                    treasury: v,
                    pools: Value::zero(),
                }
            }
        }
    }
}