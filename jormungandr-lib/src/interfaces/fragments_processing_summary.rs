use chain_impl_mockchain::fragment::FragmentId;

use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};

#[derive(Debug, Serialize, Deserialize)]
pub enum FragmentRejectionReason {
    FragmentAlreadyInLog,
    FragmentInvalid,
    PreviousFragmentInvalid,
    PoolOverflow { pool_number: usize },
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
pub struct RejectedFragmentInfo {
    #[serde_as(as = "DisplayFromStr")]
    pub id: FragmentId,
    pub reason: FragmentRejectionReason,
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
pub struct FragmentsProcessingSummary {
    #[serde_as(as = "Vec<DisplayFromStr>")]
    pub accepted: Vec<FragmentId>,
    pub rejected: Vec<RejectedFragmentInfo>,
}
