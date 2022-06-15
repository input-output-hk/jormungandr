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
    /// This marks the fragment is coming from the JRpc interface
    /// (a client wallet or another service).
    JRpc,
}

/// status of the fragment within the blockchain or the pool
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum FragmentStatus {
    /// the fragment is yet to be processed
    Pending,
    /// the fragment has been rejected and won't be added in a block
    Rejected { reason: String },
    /// The fragment has been added in a block
    InABlock { date: BlockDate, block: Hash },
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

impl FragmentStatus {
    #[inline]
    pub fn is_pending(&self) -> bool {
        self == &FragmentStatus::Pending
    }

    #[inline]
    pub fn is_rejected(&self) -> bool {
        matches!(self, FragmentStatus::Rejected { .. })
    }

    #[inline]
    pub fn is_in_a_block(&self) -> bool {
        matches!(self, FragmentStatus::InABlock { .. })
    }
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
        self.status().is_pending()
    }

    #[inline]
    pub fn is_rejected(&self) -> bool {
        self.status().is_rejected()
    }

    #[inline]
    pub fn is_in_a_block(&self) -> bool {
        self.status().is_in_a_block()
    }

    /// Set the new status
    ///
    /// # Returns
    ///
    /// `true` if the value was updated. `false` if the upadte was refused.
    #[inline]
    pub fn modify(&mut self, new_status: FragmentStatus) -> bool {
        // we must not be able to transition from InABlock to Pending or Rejected
        if self.status.is_in_a_block() && !new_status.is_in_a_block() {
            return false;
        }
        self.status = new_status;
        self.last_updated_at = SystemTime::now();
        true
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
