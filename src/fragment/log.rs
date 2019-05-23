use crate::{blockcfg::BlockDate, fragment::FragmentId};
use jormungandr_utils::serde;
use serde::Serialize;
use std::time::SystemTime;

/// identify the source of a fragment
#[derive(Copy, Clone, Serialize, Debug)]
pub enum Origin {
    /// trace back the origin of a fragment to a given
    /// network node. This will allow tracking back the
    /// origins of the fragment and eventually blacklisting
    /// the senders from sending us more fragment (in case
    /// they are invalids or so)
    ///
    /// TODO: add the network identifier/IP Address
    Network,
    /// This marks the fragment is coming from the REST interface
    /// (a client wallet or another service).
    Rest,
}

/// status of the fragment within the blockchain or the pool
#[derive(Clone, Serialize)]
pub enum Status {
    /// the fragment is yet to be processed
    Pending,
    /// the fragment has been rejected and won't be added in a block
    Rejected { reason: String },
    /// The fragment has been added in a block
    #[serde(with = "serde::as_string")]
    InABlock { date: BlockDate },
}

/// the log associated to a given fragment
#[derive(Clone, Serialize)]
pub struct Log {
    #[serde(with = "serde::as_string")]
    pub fragment_id: FragmentId,
    pub last_updated_at: SystemTime,
    pub received_at: SystemTime,
    pub received_from: Origin,
    pub status: Status,
}
