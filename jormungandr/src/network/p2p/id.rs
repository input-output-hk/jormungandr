use crate::network::p2p::limits;
use bincode;
use chain_core::property;
use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};

/// a P2P node identifier
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Id(poldercast::Address);

impl fmt::Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl FromStr for Id {
    type Err = <poldercast::Address as FromStr>::Err;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse().map(Id)
    }
}

impl From<poldercast::Address> for Id {
    fn from(id: poldercast::Address) -> Self {
        Id(id)
    }
}

impl From<Id> for poldercast::Address {
    fn from(id: Id) -> Self {
        id.0
    }
}

impl property::Serialize for Id {
    type Error = bincode::Error;

    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        let mut config = bincode::config();
        config.limit(limits::MAX_ID_SIZE);

        config.serialize_into(writer, &self.0)
    }
}

impl property::Deserialize for Id {
    type Error = bincode::Error;

    fn deserialize<R: std::io::BufRead>(reader: R) -> Result<Self, Self::Error> {
        let mut config = bincode::config();
        config.limit(limits::MAX_ID_SIZE);

        config.deserialize_from(reader).map(Id)
    }
}
