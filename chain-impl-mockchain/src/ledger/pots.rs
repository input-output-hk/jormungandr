use crate::ledger::Error;
use crate::value::Value;

/// Special pots of money
#[derive(Clone, PartialEq, Eq)]
pub struct Pots {
    pub(crate) fees: Value,
}

#[derive(Debug, Clone, Copy)]
pub enum Entry {
    Fees(Value),
}

impl Pots {
    /// Create a new empty set of pots
    pub fn zero() -> Self {
        Pots {
            fees: Value::zero(),
        }
    }

    /// Sum the total values in the pots
    pub fn total_value(&self) -> Value {
        self.fees
    }

    /// Append some fees in the pots
    pub fn append_fees(&mut self, fees: Value) -> Result<(), Error> {
        self.fees = (self.fees + fees).map_err(|error| Error::PotValueInvalid { error })?;
        Ok(())
    }

    pub fn entries(&self) -> Vec<Entry> {
        vec![Entry::Fees(self.fees)]
    }

    pub fn from_entries(ents: &[Entry]) -> Self {
        let mut pots = Pots::zero();
        for e in ents {
            match e {
                Entry::Fees(v) => pots.fees = *v,
            }
        }
        pots
    }
}
