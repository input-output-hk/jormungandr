use chain_impl_mockchain::fragment::{Fragment, FragmentId};
use jormungandr_lib::{
    crypto::hash::Hash,
    interfaces::{BlockDate, FragmentLog},
};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum FragmentNodeError {
    #[error("cannot send fragment due to '{reason}' to '{fragment_id}' to node '{alias}' . logs: {logs}")]
    CannotSendFragment {
        reason: String,
        alias: String,
        fragment_id: FragmentId,
        logs: String,
    },
    #[error("reqwest error")]
    ReqwestError(#[from] reqwest::Error),
    #[error("unknown error")]
    UnknownError,
    #[error("cannot list fragments error due to '{0}'")]
    ListFragmentError(String),
}

pub trait FragmentNode {
    fn alias(&self) -> &str;
    fn fragment_logs(&self) -> Result<HashMap<FragmentId, FragmentLog>, FragmentNodeError>;
    fn send_fragment(&self, fragment: Fragment) -> Result<MemPoolCheck, FragmentNodeError>;
    fn log_pending_fragment(&self, fragment_id: FragmentId);
    fn log_rejected_fragment(&self, fragment_id: FragmentId, reason: String);
    fn log_in_block_fragment(&self, fragment_id: FragmentId, date: BlockDate, block: Hash);
    fn log_content(&self) -> String;
}

#[derive(Clone, Debug)]
pub struct MemPoolCheck {
    fragment_id: FragmentId,
}

impl MemPoolCheck {
    pub fn new(fragment_id: FragmentId) -> Self {
        Self {
            fragment_id: fragment_id,
        }
    }

    pub fn fragment_id(&self) -> &FragmentId {
        &self.fragment_id
    }
}
