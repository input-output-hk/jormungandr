use crate::testing::fragments::node::{FragmentNode, FragmentNodeError, MemPoolCheck};
use chain_impl_mockchain::fragment::FragmentId;
use custom_debug::CustomDebug;
use jormungandr_lib::interfaces::FragmentStatus;
use std::time::Duration;
use thiserror::Error;

#[derive(Error, CustomDebug)]
pub enum FragmentVerifierError {
    #[error("fragment sent to node: {alias} is not in block :({status:?})")]
    FragmentNotInBlock {
        alias: String,
        status: FragmentStatus,
        #[debug(skip)]
        logs: Vec<String>,
    },
    #[error("transaction already balanced")]
    FragmentIsPendingForTooLong {
        fragment_id: FragmentId,
        timeout: Duration,
        alias: String,
        #[debug(skip)]
        logs: Vec<String>,
    },
    #[error("fragment sent to node: {alias} is not in in fragment pool :({fragment_id})")]
    FragmentNotInMemPoolLogs {
        alias: String,
        fragment_id: FragmentId,
        #[debug(skip)]
        logs: Vec<String>,
    },
    #[error("fragment node error")]
    FragmentNode(#[from] FragmentNodeError),
}

impl FragmentVerifierError {
    pub fn logs(&self) -> impl Iterator<Item = &str> {
        use self::FragmentVerifierError::*;
        let maybe_logs = match self {
            FragmentNotInBlock { logs, .. }
            | FragmentIsPendingForTooLong { logs, .. }
            | FragmentNotInMemPoolLogs { logs, .. }
            | FragmentNode(FragmentNodeError::CannotSendFragment { logs, .. }) => Some(logs),
            FragmentNode(_) => None,
        };
        maybe_logs
            .into_iter()
            .map(|logs| logs.iter())
            .flatten()
            .map(String::as_str)
    }
}

pub struct FragmentVerifier;

impl FragmentVerifier {
    pub fn wait_and_verify_is_in_block<A: FragmentNode + ?Sized>(
        &self,
        duration: Duration,
        check: MemPoolCheck,
        node: &A,
    ) -> Result<(), FragmentVerifierError> {
        let status = self.wait_fragment(duration, check, node)?;
        self.is_in_block(status, node)
    }

    pub fn is_in_block<A: FragmentNode + ?Sized>(
        &self,
        status: FragmentStatus,
        node: &A,
    ) -> Result<(), FragmentVerifierError> {
        if !status.is_in_a_block() {
            return Err(FragmentVerifierError::FragmentNotInBlock {
                alias: node.alias().to_string(),
                status,
                logs: node.log_content(),
            });
        }
        Ok(())
    }

    pub fn fragment_status<A: FragmentNode + ?Sized>(
        &self,
        check: MemPoolCheck,
        node: &A,
    ) -> Result<FragmentStatus, FragmentVerifierError> {
        let logs = node.fragment_logs()?;
        if let Some(log) = logs.get(check.fragment_id()) {
            let status = log.status().clone();
            match log.status() {
                FragmentStatus::Pending => {
                    node.log_pending_fragment(check.fragment_id().clone());
                }
                FragmentStatus::Rejected { reason } => {
                    node.log_rejected_fragment(check.fragment_id().clone(), reason.to_string());
                }
                FragmentStatus::InABlock { date, block } => {
                    node.log_in_block_fragment(
                        check.fragment_id().clone(),
                        date.clone(),
                        block.clone(),
                    );
                }
            }
            return Ok(status);
        }

        Err(FragmentVerifierError::FragmentNotInMemPoolLogs {
            alias: node.alias().to_string(),
            fragment_id: *check.fragment_id(),
            logs: node.log_content(),
        })
    }

    pub fn wait_fragment<A: FragmentNode + ?Sized>(
        &self,
        duration: Duration,
        check: MemPoolCheck,
        node: &A,
    ) -> Result<FragmentStatus, FragmentVerifierError> {
        let max_try = 50;
        for _ in 0..max_try {
            let status_result = self.fragment_status(check.clone(), node);

            if status_result.is_err() {
                std::thread::sleep(duration);
                continue;
            }

            let status = status_result.unwrap();

            match status {
                FragmentStatus::Rejected { .. } => return Ok(status),
                FragmentStatus::InABlock { .. } => return Ok(status),
                _ => (),
            }
            std::thread::sleep(duration);
        }

        Err(FragmentVerifierError::FragmentIsPendingForTooLong {
            fragment_id: *check.fragment_id(),
            timeout: Duration::from_secs(duration.as_secs() * max_try),
            alias: node.alias().to_string(),
            logs: node.log_content(),
        })
    }
}
