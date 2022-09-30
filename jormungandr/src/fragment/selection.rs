use super::{logs::Logs, pool::internal::Pool};
use crate::{
    blockcfg::{ApplyBlockLedger, Contents, ContentsBuilder},
    fragment::{Fragment, FragmentId},
};
use async_trait::async_trait;
use chain_core::property::Serialize;
use futures::{channel::oneshot::Receiver, future::Shared, prelude::*};
use jormungandr_lib::interfaces::{BlockDate, FragmentStatus};
use std::{error::Error, iter};
use tracing::{debug_span, Instrument};

pub enum SelectionOutput {
    Commit { fragment_id: FragmentId },
    RequestSmallerFee,
    RequestSmallerSize,
    Reject { reason: String },
}

#[async_trait]
pub trait FragmentSelectionAlgorithm {
    async fn select(
        &mut self,
        ledger: ApplyBlockLedger,
        logs: &mut Logs,
        pool: &mut Pool,
        soft_deadline_future: futures::channel::oneshot::Receiver<()>,
        hard_deadline_future: futures::channel::oneshot::Receiver<()>,
    ) -> FragmentSelectionResult;
}

pub struct FragmentSelectionResult {
    pub contents: Contents,
    pub ledger: ApplyBlockLedger,
    pub rejected_fragments_cnt: usize,
}

#[derive(Debug)]
pub enum FragmentSelectionAlgorithmParams {
    OldestFirst,
}

pub struct OldestFirst;

impl OldestFirst {
    pub fn new() -> Self {
        OldestFirst
    }
}

impl Default for OldestFirst {
    fn default() -> Self {
        Self::new()
    }
}

enum ApplyFragmentError {
    DoesNotFit,
    SoftDeadlineReached,
    Rejected(String),
}

struct NewLedgerState {
    ledger: ApplyBlockLedger,
    space_left: u32,
}

async fn try_apply_fragment(
    fragment: Fragment,
    ledger: ApplyBlockLedger,
    soft_deadline_future: Shared<Receiver<()>>,
    hard_deadline_future: Shared<Receiver<()>>,
    mut space_left: u32,
) -> Result<NewLedgerState, ApplyFragmentError> {
    use futures::future::{select, Either};

    let raw_fragment_size = fragment.serialized_size();
    let block_content_max_size = ledger.settings().block_content_max_size;
    let fragment_size = match u32::try_from(raw_fragment_size) {
        Ok(size) if size <= block_content_max_size => size,
        _ => {
            let reason = format!(
                "fragment size {} exceeds maximum block content size {}",
                raw_fragment_size, block_content_max_size
            );
            return Err(ApplyFragmentError::Rejected(reason));
        }
    };

    if fragment_size > space_left {
        // return a fragment to the pool later if does not fit the contents size limit
        tracing::trace!("discarding fragment that does not fit in block");
        return Err(ApplyFragmentError::DoesNotFit);
    }

    space_left -= fragment_size;

    tracing::debug!("applying fragment in simulation");

    let fragment_future = tokio::task::spawn_blocking(move || ledger.apply_fragment(&fragment));

    let ledger_res = match select(fragment_future, soft_deadline_future.clone()).await {
        Either::Left((join_result, _)) => join_result.unwrap(),
        Either::Right((_, fragment_future)) => {
            if space_left < block_content_max_size {
                tracing::debug!(
                    "aborting processing of the current fragment to satisfy the soft deadline"
                );
                return Err(ApplyFragmentError::SoftDeadlineReached);
            }

            tracing::debug!(
                "only one fragment in progress: continuing until meeting the hard deadline"
            );

            match select(fragment_future, hard_deadline_future.clone()).await {
                Either::Left((join_result, _)) => join_result.unwrap(),
                Either::Right(_) => return Err(ApplyFragmentError::Rejected(
                    "cannot process a single fragment within the given time bounds (hard deadline)"
                        .into(),
                )),
            }
        }
    };

    match ledger_res {
        Ok(ledger) => Ok(NewLedgerState { ledger, space_left }),
        Err(err) => {
            let mut msg = err.to_string();
            for e in iter::successors(err.source(), |&e| e.source()) {
                msg.push_str(": ");
                msg.push_str(&e.to_string());
            }
            Err(ApplyFragmentError::Rejected(msg))
        }
    }
}

#[async_trait]
impl FragmentSelectionAlgorithm for OldestFirst {
    async fn select(
        &mut self,
        mut ledger: ApplyBlockLedger,
        logs: &mut Logs,
        pool: &mut Pool,
        soft_deadline_future: futures::channel::oneshot::Receiver<()>,
        hard_deadline_future: futures::channel::oneshot::Receiver<()>,
    ) -> FragmentSelectionResult {
        let date: BlockDate = ledger.block_date().into();
        let mut space_left = ledger.settings().block_content_max_size;
        let mut contents_builder = ContentsBuilder::new();
        let mut return_to_pool = Vec::new();
        let mut rejected_fragments_cnt = 0;

        let soft_deadline_future = soft_deadline_future.shared();
        let hard_deadline_future = hard_deadline_future.shared();
        while let Some((fragment, id)) = pool.remove_oldest() {
            let span = debug_span!("fragment", hash=%id.to_string());

            async {
                let result = try_apply_fragment(
                    fragment.clone(),
                    ledger.clone(),
                    soft_deadline_future.clone(),
                    hard_deadline_future.clone(),
                    space_left,
                )
                .await;
                match result {
                    Ok(NewLedgerState {
                        ledger: ledger_new,
                        space_left: space_left_new,
                    }) => {
                        contents_builder.push(fragment);
                        ledger = ledger_new;
                        tracing::debug!("successfully applied and committed the fragment");
                        space_left = space_left_new;
                    }
                    Err(ApplyFragmentError::DoesNotFit)
                    | Err(ApplyFragmentError::SoftDeadlineReached) => {
                        return_to_pool.push((fragment, id));
                    }
                    Err(ApplyFragmentError::Rejected(reason)) => {
                        tracing::debug!(%reason, "fragment is rejected");
                        logs.modify(id, FragmentStatus::Rejected { reason }, date);
                        rejected_fragments_cnt += 1;
                    }
                }
            }
            .instrument(span)
            .await;

            if space_left == 0 {
                tracing::debug!("block has reached max total size, exiting");
                break;
            }
        }

        tracing::debug!(
            "finished block creation with {} fragments left in the pool",
            pool.len()
        );
        return_to_pool.reverse();
        pool.return_to_pool(return_to_pool);

        FragmentSelectionResult {
            contents: contents_builder.into(),
            ledger,
            rejected_fragments_cnt,
        }
    }
}
