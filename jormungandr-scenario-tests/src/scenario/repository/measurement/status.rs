use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MeasurementStatus {
    Green,
    Yellow,
    Red,
}

impl fmt::Display for MeasurementStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
