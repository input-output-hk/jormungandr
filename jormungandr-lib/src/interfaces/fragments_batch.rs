use crate::interfaces::FragmentDef;
use chain_impl_mockchain::fragment::Fragment;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

/// Submission of a batch of fragments to the node.
#[serde_as]
#[derive(Debug, Deserialize, Serialize)]
pub struct FragmentsBatch {
    /// Fragments are processed in the sequential order. When this option is enabled, fragments
    /// processing will stop upon meeting the first invalid fragment and the rest of fragments
    /// would be dropped.
    pub fail_fast: bool,
    /// Fragments themselves.
    #[serde_as(as = "Vec<FragmentDef>")]
    pub fragments: Vec<Fragment>,
}
