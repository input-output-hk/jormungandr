use crate::block::Epoch;
use crate::value::{Value, ValueError};
use chain_core::mempack::{ReadBuf, ReadError};
use std::num::NonZeroU64;
use typed_bytes::ByteBuilder;

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

impl TaxType {
    pub fn zero() -> Self {
        TaxType {
            fixed: Value(0),
            ratio: Ratio { numerator: 0, denominator: NonZeroU64::new(1).unwrap() },
            max_limit: None
        }
    }

    pub fn serialize_in(&self, bb: ByteBuilder<Self>) -> ByteBuilder<Self> {
        bb.u64(self.fixed.0)
            .u64(self.ratio.numerator)
            .u64(self.ratio.denominator.get())
            .u64(self.max_limit.map_or(0, |v| v.get()))
    }

    pub fn read_frombuf(rb: &mut ReadBuf) -> Result<Self, ReadError> {
        let fixed = rb.get_u64().map(Value)?;
        let num = rb.get_u64()?;
        let denom = rb.get_u64()?;
        let limit = rb.get_u64()?;
        let denominator = NonZeroU64::new(denom).map_or_else(
            || {
                Err(ReadError::StructureInvalid(
                    "ratio fraction divisor invalid".to_string(),
                ))
            },
            Ok,
        )?;
        if num > denom {
            return Err(ReadError::StructureInvalid(
                "ratio fraction invalid bigger than 1".to_string(),
            ));
        }

        Ok(TaxType {
            fixed,
            ratio: Ratio {
                numerator: num,
                denominator,
            },
            max_limit: NonZeroU64::new(limit),
        })
    }
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

/// A value distributed between tax and remaining
#[derive(Debug, Clone)]
pub struct TaxDistribution {
    pub taxed: Value,
    pub after_tax: Value,
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

/// Tax some value into the tax value and what is remaining
pub fn tax_cut(v: Value, tax_type: &TaxType) -> Result<TaxDistribution, ValueError> {
    let mut left = v;
    let mut taxed = Value::zero();

    // subtract fix amount
    match left - tax_type.fixed {
        Ok(left1) => {
            left = left1;
            taxed = (taxed + tax_type.fixed)?;
        }
        Err(_) => {
            return Ok(TaxDistribution {
                taxed: v,
                after_tax: Value::zero(),
            })
        }
    };

    // calculate and subtract ratio
    {
        let rr = tax_type.ratio;
        let olimit = tax_type.max_limit;

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
                taxed = (taxed + treasury_cut)?;
            }
            Err(_) => {
                left = Value::zero();
                taxed = (taxed + left)?;
            }
        }
    };

    Ok(TaxDistribution {
        taxed,
        after_tax: left,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use quickcheck::{Arbitrary, Gen, TestResult};
    use quickcheck_macros::quickcheck;

    #[quickcheck]
    fn tax_cut_fully_accounted(v: Value, treasury_tax: TaxType) -> TestResult {
        match tax_cut(v, &treasury_tax) {
            Ok(td) => {
                let sum = (td.taxed + td.after_tax).unwrap();
                if sum == v {
                    TestResult::passed()
                } else {
                    TestResult::error(format!(
                        "mismatch taxed={} remaining={} expected={} got={} for {:?}",
                        td.taxed, td.after_tax, v, sum, treasury_tax
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
