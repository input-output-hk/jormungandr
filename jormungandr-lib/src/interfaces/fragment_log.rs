use crate::{crypto::hash::Hash, interfaces::BlockDate, time::SystemTime};
use chain_impl_mockchain::key;
use serde::{Deserialize, Serialize};

/// identify the source of a fragment
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum FragmentOrigin {
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
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum FragmentStatus {
    /// the fragment is yet to be processed
    Pending,
    /// the fragment has been rejected and won't be added in a block
    Rejected { reason: String },
    /// The fragment has been added in a block
    InABlock { date: BlockDate },
}

/// the log associated to a given fragment
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct FragmentLog {
    fragment_id: Hash,
    received_from: FragmentOrigin,
    received_at: SystemTime,
    last_updated_at: SystemTime,
    status: FragmentStatus,
}

impl FragmentLog {
    /// create a new FragmentLog with the given values
    #[inline]
    pub fn new(fragment_id: key::Hash, received_from: FragmentOrigin) -> Self {
        FragmentLog {
            fragment_id: fragment_id.into(),
            received_from,
            received_at: SystemTime::now(),
            last_updated_at: SystemTime::now(),
            status: FragmentStatus::Pending,
        }
    }

    #[inline]
    pub fn is_pending(&self) -> bool {
        self.status == FragmentStatus::Pending
    }

    #[inline]
    pub fn is_rejected(&self) -> bool {
        if let FragmentStatus::Rejected { .. } = &self.status {
            true
        } else {
            false
        }
    }

    #[inline]
    pub fn is_in_a_block(&self) -> bool {
        if let FragmentStatus::InABlock { .. } = &self.status {
            true
        } else {
            false
        }
    }

    /// set the new status
    #[inline]
    pub fn modify(&mut self, new_status: FragmentStatus) {
        self.status = new_status;
        self.last_updated_at = SystemTime::now();
    }

    #[inline]
    pub fn fragment_id(&self) -> &Hash {
        &self.fragment_id
    }

    #[inline]
    pub fn received_from(&self) -> &FragmentOrigin {
        &self.received_from
    }

    #[inline]
    pub fn received_at(&self) -> &SystemTime {
        &self.received_at
    }

    #[inline]
    pub fn last_updated_at(&self) -> &SystemTime {
        &self.last_updated_at
    }

    #[inline]
    pub fn status(&self) -> &FragmentStatus {
        &self.status
    }
}
