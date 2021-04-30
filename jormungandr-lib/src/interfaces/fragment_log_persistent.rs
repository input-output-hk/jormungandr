use crate::interfaces::FragmentDef;
use crate::time::SecondsSinceUnixEpoch;

use chain_impl_mockchain::fragment::Fragment;

use serde::{Deserialize, Serialize};

/// Represents a persistent fragments log entry.
#[derive(Debug, Serialize, Deserialize)]
pub struct PersistentFragmentLog {
    /// the time this fragment was registered and accepted by the pool
    pub time: SecondsSinceUnixEpoch,
    /// full hex-encoded fragment body
    #[serde(with = "FragmentDef")]
    pub fragment: Fragment,
}
