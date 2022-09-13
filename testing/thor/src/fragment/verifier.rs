use chain_impl_mockchain::fragment::FragmentId;
use jormungandr_automation::jormungandr::{FragmentNode, FragmentNodeError, MemPoolCheck};
use jormungandr_lib::interfaces::{FragmentLog, FragmentStatus};
use jortestkit::prelude::Wait;
use std::{collections::HashMap, time::Duration};

#[derive(custom_debug::Debug, thiserror::Error)]
pub enum FragmentVerifierError {
    #[error("fragment sent to node: {alias} is not in block :({status:?})")]
    FragmentNotInBlock {
        alias: String,
        status: FragmentStatus,
        #[debug(skip)]
        logs: Vec<String>,
    },
    #[error("cannot match rejection reason '{message}' does not contains '{expected_part}'")]
    UnexpectedRejectionReason {
        message: String,
        expected_part: String,
    },
    #[error("fragment sent to node: {alias} is not rejected :({status:?})")]
    FragmentNotRejected {
        alias: String,
        status: FragmentStatus,
        #[debug(skip)]
        logs: Vec<String>,
    },
    #[error("transaction is pending for too long")]
    FragmentIsPendingForTooLong {
        fragment_id: FragmentId,
        timeout: Duration,
        alias: String,
        #[debug(skip)]
        logs: Vec<String>,
    },
    #[error("transactions are pending for too long")]
    FragmentsArePendingForTooLong {
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
    #[error("at least on rejected fragment error")]
    AtLeastOneRejectedFragment {
        fragment_id: FragmentId,
        #[debug(skip)]
        logs: Vec<String>,
    },
    #[error("timeout reached while waiting for all fragments in a block")]
    TimeoutReachedWhileWaitingForAllFragmentsInBlock {
        #[debug(skip)]
        logs: Vec<String>,
    },
}

impl FragmentVerifierError {
    pub fn logs(&self) -> impl Iterator<Item = &str> {
        use self::FragmentVerifierError::*;
        let maybe_logs = match self {
            FragmentNotInBlock { logs, .. }
            | FragmentIsPendingForTooLong { logs, .. }
            | FragmentsArePendingForTooLong { logs, .. }
            | FragmentNotInMemPoolLogs { logs, .. }
            | FragmentNotRejected { logs, .. }
            | FragmentNode(FragmentNodeError::CannotSendFragment { logs, .. }) => Some(logs),
            AtLeastOneRejectedFragment { logs, .. } => Some(logs),
            TimeoutReachedWhileWaitingForAllFragmentsInBlock { logs } => Some(logs),
            FragmentNode(_) => None,
            UnexpectedRejectionReason { .. } => None,
        };
        maybe_logs
            .into_iter()
            .flat_map(|logs| logs.iter())
            .map(String::as_str)
    }
}

pub struct FragmentVerifier;

impl FragmentVerifier {
    pub fn wait_until_all_processed<A: FragmentNode + ?Sized>(
        wait: Wait,
        node: &A,
    ) -> Result<(), FragmentVerifierError> {
        for _ in 0..wait.attempts() {
            let fragment_logs = match node.fragment_logs() {
                Err(_) => {
                    std::thread::sleep(wait.sleep_duration());
                    continue;
                }
                Ok(fragment_logs) => fragment_logs,
            };

            if let Some((id, _)) = fragment_logs.iter().find(|(_, x)| x.is_rejected()) {
                return Err(FragmentVerifierError::AtLeastOneRejectedFragment {
                    fragment_id: *id,
                    logs: node.log_content(),
                });
            }

            if fragment_logs.iter().all(|(_, x)| x.is_in_a_block()) {
                return Ok(());
            }
            std::thread::sleep(wait.sleep_duration());
        }
        Err(
            FragmentVerifierError::TimeoutReachedWhileWaitingForAllFragmentsInBlock {
                logs: node.log_content(),
            },
        )
    }

    pub fn wait_and_verify_all_are_in_block<A: FragmentNode + ?Sized>(
        duration: Duration,
        checks: Vec<MemPoolCheck>,
        node: &A,
    ) -> Result<(), FragmentVerifierError> {
        for check in checks {
            let status = Self::wait_fragment(duration, check, Default::default(), node)?;
            Self::is_in_block(status, node)?;
        }
        Ok(())
    }

    pub fn wait_and_verify_is_in_block<A: FragmentNode + ?Sized>(
        duration: Duration,
        check: MemPoolCheck,
        node: &A,
    ) -> Result<(), FragmentVerifierError> {
        let status = Self::wait_fragment(duration, check, Default::default(), node)?;
        Self::is_in_block(status, node)
    }

    pub fn wait_and_verify_is_rejected<A: FragmentNode + ?Sized>(
        duration: Duration,
        check: MemPoolCheck,
        node: &A,
    ) -> Result<(), FragmentVerifierError> {
        let status = Self::wait_fragment(duration, check, Default::default(), node)?;
        Self::is_rejected(status, node)
    }

    pub fn wait_and_verify_is_rejected_with_message<A: FragmentNode + ?Sized, S: Into<String>>(
        duration: Duration,
        check: MemPoolCheck,
        message: S,
        node: &A,
    ) -> Result<(), FragmentVerifierError> {
        let status = Self::wait_fragment(duration, check, Default::default(), node)?;
        Self::is_rejected_with_message(status, message, node)
    }

    pub fn is_in_block<A: FragmentNode + ?Sized>(
        status: FragmentStatus,
        node: &A,
    ) -> Result<(), FragmentVerifierError> {
        if !status.is_in_a_block() {
            return Err(FragmentVerifierError::FragmentNotInBlock {
                alias: node.alias(),
                status,
                logs: node.log_content(),
            });
        }
        Ok(())
    }

    pub fn is_rejected<A: FragmentNode + ?Sized>(
        status: FragmentStatus,
        node: &A,
    ) -> Result<(), FragmentVerifierError> {
        if !status.is_rejected() {
            return Err(FragmentVerifierError::FragmentNotRejected {
                alias: node.alias(),
                status,
                logs: node.log_content(),
            });
        }
        Ok(())
    }

    pub fn is_rejected_with_message<A: FragmentNode + ?Sized, S: Into<String>>(
        status: FragmentStatus,
        expected_part: S,
        node: &A,
    ) -> Result<(), FragmentVerifierError> {
        if let FragmentStatus::Rejected { reason } = status {
            let expected_part = expected_part.into();
            reason.contains(&expected_part).then_some(()).ok_or(
                FragmentVerifierError::UnexpectedRejectionReason {
                    message: reason,
                    expected_part,
                },
            )
        } else {
            Err(FragmentVerifierError::FragmentNotRejected {
                alias: node.alias(),
                status,
                logs: node.log_content(),
            })
        }
    }

    pub fn fragment_status<A: FragmentNode + ?Sized>(
        check: MemPoolCheck,
        node: &A,
    ) -> Result<FragmentStatus, FragmentVerifierError> {
        let logs = node.fragment_logs()?;
        if let Some(log) = logs.get(check.fragment_id()) {
            let status = log.status().clone();
            match log.status() {
                FragmentStatus::Pending => {
                    node.log_pending_fragment(*check.fragment_id());
                }
                FragmentStatus::Rejected { reason } => {
                    node.log_rejected_fragment(*check.fragment_id(), reason.to_string());
                }
                FragmentStatus::InABlock { date, block } => {
                    node.log_in_block_fragment(*check.fragment_id(), *date, *block);
                }
            }
            return Ok(status);
        }

        Err(FragmentVerifierError::FragmentNotInMemPoolLogs {
            alias: node.alias(),
            fragment_id: *check.fragment_id(),
            logs: node.log_content(),
        })
    }

    pub fn wait_fragment<A: FragmentNode + ?Sized>(
        duration: Duration,
        check: MemPoolCheck,
        exit_strategy: ExitStrategy,
        node: &A,
    ) -> Result<FragmentStatus, FragmentVerifierError> {
        let max_try = 50;
        for _ in 0..max_try {
            let status_result = Self::fragment_status(check.clone(), node);

            if status_result.is_err() {
                std::thread::sleep(duration);
                continue;
            }

            let status = status_result.unwrap();

            match (&status, exit_strategy) {
                (FragmentStatus::Rejected { .. }, _) => return Ok(status),
                (FragmentStatus::InABlock { .. }, _) => return Ok(status),
                (FragmentStatus::Pending, ExitStrategy::OnPending) => return Ok(status),
                _ => (),
            }
            std::thread::sleep(duration);
        }

        Err(FragmentVerifierError::FragmentIsPendingForTooLong {
            fragment_id: *check.fragment_id(),
            timeout: Duration::from_secs(duration.as_secs() * max_try),
            alias: node.alias(),
            logs: node.log_content(),
        })
    }

    pub fn wait_for_all_fragments<A: FragmentNode + ?Sized>(
        duration: Duration,
        node: &A,
    ) -> Result<HashMap<FragmentId, FragmentLog>, FragmentVerifierError> {
        let max_try = 50;
        for _ in 0..max_try {
            let status_result = node.fragment_logs();

            if status_result.is_err() {
                std::thread::sleep(duration);
                continue;
            }

            let statuses = status_result.unwrap();

            let any_rejected = statuses.iter().any(|(_, log)| {
                !matches!(
                    log.status(),
                    FragmentStatus::Rejected { .. } | FragmentStatus::InABlock { .. }
                )
            });

            if !any_rejected {
                return Ok(statuses);
            }
            std::thread::sleep(duration);
        }

        Err(FragmentVerifierError::FragmentsArePendingForTooLong {
            timeout: Duration::from_secs(duration.as_secs() * max_try),
            alias: node.alias(),
            logs: node.log_content(),
        })
    }
}

#[derive(Clone, Copy)]
pub enum ExitStrategy {
    /// Exit as soon as the fragment enters the mempool
    OnPending,
    /// Exit when the fragment has been processed (i.e. either Rejected or InABlock)
    OnProcessed,
}

impl Default for ExitStrategy {
    fn default() -> Self {
        ExitStrategy::OnProcessed
    }
}
