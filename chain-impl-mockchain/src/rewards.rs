use crate::block::Epoch;
use crate::value::{Value, ValueError};
use std::num::NonZeroU64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReducingType {
    Linear,
    Halvening,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ratio {
    pub numerator: u64,
    pub denominator: NonZeroU64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TaxType {
    // what get subtracted as fixed value
    pub fixed: Value,
    // Ratio of tax after fixed amout subtracted
    pub ratio: Ratio,
    // Max limit of tax
    pub max_limit: Option<NonZeroU64>,
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
pub struct TreasuryDistribution {
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
            let reduce_by = (rr.numerator * zone) / rr.denominator.get();
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
            const SCALE: u128 = 10 ^ 18;

            let mut acc = params.rewards_initial_value as u128 * SCALE;
            for _ in 0..zone {
                acc *= rr.numerator as u128;
                acc /= rr.denominator.get() as u128;
            }

            Value((acc / SCALE) as u64)
        }
    }
}

/// Distribute a pot of value to treasury and pools according to redistribution parameters
pub fn treasury_cut(v: Value, treasury_tax: &TaxType) -> Result<TreasuryDistribution, ValueError> {
    let mut left = v;
    let mut tax = Value::zero();

    // subtract fix amount
    match left - treasury_tax.fixed {
        Ok(left1) => {
            left = left1;
            tax = (tax + treasury_tax.fixed)?;
        }
        Err(_) => {
            return Ok(TreasuryDistribution {
                treasury: v,
                pools: Value::zero(),
            })
        }
    };

    // calculate and subtract ratio
    {
        let rr = treasury_tax.ratio;
        let olimit = treasury_tax.max_limit;

        const SCALE: u128 = 10 ^ 9;
        let out = ((((left.0 as u128 * SCALE) * rr.numerator as u128)
            / rr.denominator.get() as u128)
            / SCALE) as u64;
        let treasury_cut = match olimit {
            None => Value(out),
            Some(limit) => Value(std::cmp::min(limit.get(), out)),
        };

        match left - treasury_cut {
            Ok(left2) => {
                left = left2;
                tax = (tax + treasury_cut)?;
            }
            Err(_) => {
                left = Value::zero();
                tax = (tax + left)?;
            }
        }
    };

    Ok(TreasuryDistribution {
        treasury: tax,
        pools: left,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use quickcheck::{Arbitrary, Gen, TestResult};
    use quickcheck_macros::quickcheck;

    #[quickcheck]
    fn treasury_tax_sum_equal(v: Value, treasury_tax: TaxType) -> TestResult {
        match treasury_cut(v, &treasury_tax) {
            Ok(td) => {
                let sum = (td.pools + td.treasury).unwrap();
                if sum == v {
                    TestResult::passed()
                } else {
                    TestResult::error(format!(
                        "mismatch pools={} treasury={} expected={} got={} for {:?}",
                        td.pools, td.treasury, v, sum, treasury_tax
                    ))
                }
            }
            Err(_) => TestResult::discard(),
        }
    }

    impl Arbitrary for TaxType {
        fn arbitrary<G: Gen>(gen: &mut G) -> Self {
            let fixed = Arbitrary::arbitrary(gen);
            let denominator = u64::arbitrary(gen) + 1;
            let numerator = u64::arbitrary(gen) % denominator;
            let max_limit = NonZeroU64::new(u64::arbitrary(gen));

            TaxType {
                fixed,
                ratio: Ratio {
                    numerator,
                    denominator: NonZeroU64::new(denominator).unwrap(),
                },
                max_limit,
            }
        }
    }
}
