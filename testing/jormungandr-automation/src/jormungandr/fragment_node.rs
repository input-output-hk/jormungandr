use crate::jormungandr::NodeAlias;
use chain_impl_mockchain::fragment::{Fragment, FragmentId};
use jormungandr_lib::{
    crypto::hash::Hash,
    interfaces::{BlockDate, FragmentLog, FragmentsProcessingSummary},
};
use std::collections::HashMap;

#[derive(custom_debug::Debug, thiserror::Error)]
pub enum FragmentNodeError {
    #[error("cannot send fragment due to '{reason}' to '{fragment_id}' to node '{alias}'")]
    CannotSendFragment {
        reason: String,
        alias: String,
        fragment_id: FragmentId,
        #[debug(skip)]
        logs: Vec<String>,
    },
    #[error("reqwest error")]
    ReqwestError(#[from] reqwest::Error),
    #[error("unknown error")]
    UnknownError,
    #[error("cannot list fragments error due to '{0}'")]
    ListFragmentError(String),
    #[error(
        "cannot send one of the fragments {fragment_ids:?} due to '{reason}' to to node '{alias}'"
    )]
    CannotSendFragmentBatch {
        reason: String,
        alias: String,
        fragment_ids: Vec<FragmentId>,
        #[debug(skip)]
        logs: Vec<String>,
    },
}

impl FragmentNodeError {
    pub fn logs(&self) -> impl Iterator<Item = &str> {
        use self::FragmentNodeError::*;
        let maybe_logs = match self {
            CannotSendFragment { logs, .. } => Some(logs),
            _ => None,
        };
        maybe_logs
            .into_iter()
            .flat_map(|logs| logs.iter())
            .map(String::as_str)
    }
}

pub trait FragmentNode {
    fn alias(&self) -> NodeAlias;
    fn fragment_logs(&self) -> Result<HashMap<FragmentId, FragmentLog>, FragmentNodeError>;
    fn send_fragment(&self, fragment: Fragment) -> Result<MemPoolCheck, FragmentNodeError>;
    fn send_batch_fragments(
        &self,
        fragments: Vec<Fragment>,
        fail_fast: bool,
    ) -> Result<FragmentsProcessingSummary, FragmentNodeError>;
    fn log_pending_fragment(&self, fragment_id: FragmentId);
    fn log_rejected_fragment(&self, fragment_id: FragmentId, reason: String);
    fn log_in_block_fragment(&self, fragment_id: FragmentId, valid_until: BlockDate, block: Hash);
    fn log_content(&self) -> Vec<String>;
}

#[derive(Clone, Debug)]
pub struct MemPoolCheck {
    fragment_id: FragmentId,
}

impl MemPoolCheck {
    pub fn new(fragment_id: FragmentId) -> Self {
        Self { fragment_id }
    }

    pub fn fragment_id(&self) -> &FragmentId {
        &self.fragment_id
    }
}

impl From<FragmentId> for MemPoolCheck {
    fn from(from: FragmentId) -> Self {
        Self::new(from)
    }
}
