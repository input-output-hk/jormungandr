use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Status {
    Green,
    Yellow,
    Red,
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
