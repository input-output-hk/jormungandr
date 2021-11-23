use crate::config::SessionMode;
use std::fmt;

impl fmt::Display for SessionMode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
