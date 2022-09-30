use chain_impl_mockchain::fragment::FragmentId;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};

/// This error is reserved for fragments that were rejected by the mempool at the time of sending
/// them to mempool. If a fragment ended up being included to mempool, it will be listed in
/// fragment logs and all further errors would be listed in fragment logs as well.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "reason")]
pub enum FragmentRejectionReason {
    FragmentAlreadyInLog,
    FragmentInvalid,
    PreviousFragmentInvalid,
    PoolOverflow,
    FragmentExpired,
    FragmentValidForTooLong,
}

/// Information about a fragment rejected by the mempool. This is different from being rejected by
/// the ledger during an attempt to apply this fragment.
#[serde_as]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RejectedFragmentInfo {
    #[serde_as(as = "DisplayFromStr")]
    pub id: FragmentId,
    #[serde(flatten)]
    pub reason: FragmentRejectionReason,
}

/// The summary of an attempt to add transactions to mempool for further processing.
#[serde_as]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FragmentsProcessingSummary {
    #[serde_as(as = "Vec<DisplayFromStr>")]
    pub accepted: Vec<FragmentId>,
    pub rejected: Vec<RejectedFragmentInfo>,
}

impl FragmentRejectionReason {
    /// Should this rejection be treated as an error
    pub fn is_error(&self) -> bool {
        matches!(
            self,
            FragmentRejectionReason::FragmentInvalid
                | FragmentRejectionReason::PreviousFragmentInvalid
                | FragmentRejectionReason::PoolOverflow
        )
    }
}

impl FragmentsProcessingSummary {
    /// Whether any of rejected entries should be treated as an error.
    pub fn is_error(&self) -> bool {
        self.rejected.iter().any(|info| info.reason.is_error())
    }

    pub fn fragment_ids(&self) -> Vec<FragmentId> {
        self.rejected
            .iter()
            .map(|info| &info.id)
            .chain(self.accepted.iter())
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quickcheck::{Arbitrary, Gen};
    use quickcheck_macros::quickcheck;

    impl Arbitrary for FragmentRejectionReason {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            match g.next_u32() % 4 {
                0 => FragmentRejectionReason::FragmentAlreadyInLog,
                1 => FragmentRejectionReason::FragmentInvalid,
                2 => FragmentRejectionReason::PreviousFragmentInvalid,
                3 => FragmentRejectionReason::PoolOverflow,
                _ => unreachable!(),
            }
        }
    }

    impl Arbitrary for RejectedFragmentInfo {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            Self {
                id: Arbitrary::arbitrary(g),
                reason: Arbitrary::arbitrary(g),
            }
        }
    }

    impl Arbitrary for FragmentsProcessingSummary {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            Self {
                accepted: Arbitrary::arbitrary(g),
                rejected: Arbitrary::arbitrary(g),
            }
        }
    }

    #[quickcheck]
    fn fragments_processing_summary_serialization_sanity(
        summary: FragmentsProcessingSummary,
    ) -> bool {
        let json = serde_json::to_string(&summary).unwrap();
        let deserialized_summary = serde_json::from_str(&json).unwrap();
        summary == deserialized_summary
    }
}
