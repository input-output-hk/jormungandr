use crate::ledger::Error;
use crate::value::Value;

/// An amount of value owned by the treasury.
///
/// Right now, it doesn't have any mechanism to
/// withdraw money from, so it serves just to
/// record a monotically increasing special account.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Treasury(Value);

impl Treasury {
    /// Create a treasury with an initial value
    pub fn initial(v: Value) -> Self {
        Self(v)
    }

    /// Add some value in the treasury
    pub fn add(&mut self, v: Value) -> Result<(), Error> {
        self.0 = (self.0 + v).map_err(|error| Error::PotValueInvalid { error })?;
        Ok(())
    }

    /// Get value in the treasury
    pub fn value(&self) -> Value {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::Treasury;
    use quickcheck::{Arbitrary, Gen};

    impl Arbitrary for Treasury {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            Treasury::initial(Arbitrary::arbitrary(g))
        }
    }
}
