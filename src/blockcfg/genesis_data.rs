//! Generic Genesis data

use std::{error, fmt, io, time};
use serde_yaml;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenesisData {
    pub start_time: time::SystemTime,
    pub slot_duration: time::Duration,
    pub epoch_stability_depth: usize,
}

// TODO: details
#[derive(Debug)]
pub struct ParseError();

impl error::Error for ParseError {}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "error parsing genesis data")
    }
}

impl GenesisData {
    pub fn parse<R: io::BufRead>(reader: R) -> Result<Self, serde_yaml::Error> {
        serde_yaml::from_reader(reader)
    }
}
