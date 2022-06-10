use crate::jormungandr::{JormungandrRest, MemPoolCheck, RestError};
use chain_impl_mockchain::{fragment::FragmentId, key::Hash};
use jormungandr_lib::interfaces::{
    FragmentRejectionReason, FragmentStatus, FragmentsProcessingSummary,
};

pub struct FragmentLogVerifier {
    rest: JormungandrRest,
}

impl FragmentLogVerifier {
    pub fn new(rest: JormungandrRest) -> Self {
        Self { rest }
    }

    pub fn assert_size(self, size: usize) -> Self {
        assert_eq!(
            self.rest.fragment_logs().unwrap().len(),
            size,
            "only 1 transaction should be in fragment log"
        );
        self
    }

    pub fn assert_contains_only(self, hash: &Hash) -> Self {
        assert_eq!(
            self.rest
                .fragment_logs()
                .unwrap()
                .values()
                .next()
                .unwrap()
                .fragment_id()
                .into_hash(),
            *hash,
            "transaction not found in fragment log"
        );
        self
    }

    pub fn assert_empty(self) -> Self {
        assert_eq!(
            self.rest.fragment_logs().unwrap().len(),
            0,
            "none transactions should be in fragment log"
        );
        self
    }

    pub fn assert_all_valid(self, mem_pool_checks: &[MemPoolCheck]) -> Self {
        let ids: Vec<String> = mem_pool_checks
            .iter()
            .map(|x| x.fragment_id().to_string())
            .collect();
        let statuses = self.rest.fragments_statuses(ids.clone()).unwrap();

        assert_eq!(ids.len(), statuses.len());

        ids.iter().for_each(|id| match statuses.get(id) {
            Some(status) => self.assert_in_block(status),
            None => panic!("{} not found", id),
        });
        self
    }

    pub fn assert_valid(self, mem_pool_check: &MemPoolCheck) -> Self {
        let ids = vec![mem_pool_check.fragment_id().to_string()];

        let statuses = self.rest.fragments_statuses(ids.clone()).unwrap();

        assert_eq!(ids.len(), statuses.len());

        ids.iter().for_each(|id| match statuses.get(id) {
            Some(status) => self.assert_in_block(status),
            None => panic!("{} not found", id),
        });
        self
    }

    pub fn assert_not_exist(self, mem_pool_check: &MemPoolCheck) -> Self {
        let ids = vec![mem_pool_check.fragment_id().to_string()];

        let statuses = self.rest.fragments_statuses(ids).unwrap();

        assert_eq!(statuses.len(), 0);
        self
    }

    pub fn assert_invalid(self, mem_pool_check: &MemPoolCheck) -> Self {
        let ids = vec![mem_pool_check.fragment_id().to_string()];
        let statuses = self.rest.fragments_statuses(ids.clone()).unwrap();
        assert_eq!(ids.len(), statuses.len());

        ids.iter().for_each(|id| match statuses.get(id) {
            Some(status) => self.assert_not_in_block(status),
            None => panic!("{} not found", id),
        });
        self
    }

    pub fn assert_no_fragments(self) -> Self {
        let fragment_logs = self.rest.fragment_logs().unwrap();
        assert!(fragment_logs.is_empty());
        self
    }

    pub fn assert_in_block(&self, fragment_status: &FragmentStatus) {
        match fragment_status {
            FragmentStatus::InABlock { .. } => (),
            _ => panic!("should be in block '{:?}'", fragment_status),
        };
    }

    pub fn assert_not_in_block(&self, fragment_status: &FragmentStatus) {
        let in_block = matches!(fragment_status, FragmentStatus::InABlock { .. });
        assert!(!in_block, "should NOT be in block '{:?}'", fragment_status);
    }

    pub fn assert_invalid_id(self, id: String, prefix: &str) -> Self {
        let statuses = self.rest.fragments_statuses(vec![id.clone()]).unwrap();
        assert_eq!(1, statuses.len());

        let invalid_id = statuses.get(&id);

        match invalid_id {
            Some(status) => self.assert_not_in_block(status),
            None => panic!("Assert Error: {}", prefix),
        };

        self
    }

    pub fn assert_single_id(self, id: String, prefix: &str) -> Self {
        let statuses = self.rest.fragments_statuses(vec![id.clone()]).unwrap();

        assert_eq!(1, statuses.len());

        let alice_tx_status = statuses.get(&id);

        match alice_tx_status {
            Some(status) => self.assert_in_block(status),
            None => panic!("Assert Error: {}", prefix),
        };
        self
    }

    pub fn assert_multiple_ids(self, ids: Vec<String>, prefix: &str) -> Self {
        let statuses = self.rest.fragments_statuses(ids.clone()).unwrap();

        assert_eq!(ids.len(), statuses.len());

        ids.iter().for_each(|id| match statuses.get(id) {
            Some(status) => self.assert_in_block(status),
            None => panic!("{}", prefix),
        });
        self
    }

    pub fn assert_empty_ids(self, ids: Vec<String>, prefix: &str) -> Self {
        assert!(
            self.rest.fragments_statuses(ids).is_err(),
            "{} - expected failure",
            prefix
        );
        self
    }
}

pub fn assert_accepted_rejected(
    accepted: Vec<FragmentId>,
    rejected: Vec<(FragmentId, FragmentRejectionReason)>,
    result: Result<FragmentsProcessingSummary, RestError>,
) -> Vec<MemPoolCheck> {
    match result.err().unwrap() {
        RestError::NonSuccessErrorCode {
            checks,
            status,
            response,
        } => {
            let summary: FragmentsProcessingSummary = serde_json::from_str(&response).unwrap();
            if !rejected.is_empty() {
                assert_eq!(status, reqwest::StatusCode::BAD_REQUEST);
            }
            assert_eq!(summary.accepted, accepted);
            assert_eq!(
                summary
                    .rejected
                    .iter()
                    .map(|x| (x.id, x.reason.clone()))
                    .collect::<Vec<(FragmentId, FragmentRejectionReason)>>(),
                rejected
            );

            checks
        }
        _ => panic!("wrong error code"),
    }
}

pub fn assert_bad_request(
    result: Result<FragmentsProcessingSummary, RestError>,
) -> Vec<MemPoolCheck> {
    match result.err().unwrap() {
        RestError::NonSuccessErrorCode { status, checks, .. } => {
            assert_eq!(status, reqwest::StatusCode::BAD_REQUEST);
            checks
        }
        _ => panic!("unexcepted error"),
    }
}
