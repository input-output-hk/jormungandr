use super::logs::Logs;
use super::pool::internal::Pool;
use crate::{
    blockcfg::{BlockDate, Contents, ContentsBuilder, Ledger, LedgerParameters},
    fragment::FragmentId,
};
use chain_core::property::Fragment as _;
use jormungandr_lib::interfaces::FragmentStatus;

use tracing::{span, Level};

use std::error::Error;
use std::iter;

pub enum SelectionOutput {
    Commit { fragment_id: FragmentId },
    RequestSmallerFee,
    RequestSmallerSize,
    Reject { reason: String },
}

pub trait FragmentSelectionAlgorithm {
    fn select(
        &mut self,
        ledger: &Ledger,
        ledger_params: &LedgerParameters,
        block_date: BlockDate,
        logs: &mut Logs,
        pool: &mut Pool,
    );

    fn finalize(self) -> Contents;
}

#[derive(Debug)]
pub enum FragmentSelectionAlgorithmParams {
    OldestFirst,
}

pub struct OldestFirst {
    builder: ContentsBuilder,
    current_total_size: u32,
}

impl OldestFirst {
    pub fn new() -> Self {
        OldestFirst {
            builder: ContentsBuilder::new(),
            current_total_size: 0,
        }
    }
}

impl FragmentSelectionAlgorithm for OldestFirst {
    fn finalize(self) -> Contents {
        self.builder.into()
    }

    fn select(
        &mut self,
        ledger: &Ledger,
        ledger_params: &LedgerParameters,
        block_date: BlockDate,
        logs: &mut Logs,
        pool: &mut Pool,
    ) {
        let mut ledger_simulation = ledger.clone();

        let mut return_to_pool = Vec::new();

        while let Some(fragment) = pool.remove_oldest() {
            let id = fragment.id();
            let fragment_raw = fragment.to_raw(); // TODO: replace everything to FragmentRaw in the node
            let fragment_size = fragment_raw.size_bytes_plus_size() as u32;

            let span = span!(Level::TRACE, "fragment_selection_algorithm", kind="older_first", hash=%id.to_string());
            let _enter = span.enter();
            if fragment_size > ledger_params.block_content_max_size {
                let reason = format!(
                    "fragment size {} exceeds maximum block content size {}",
                    fragment_size, ledger_params.block_content_max_size
                );
                tracing::debug!("{}", reason);
                logs.modify(id, FragmentStatus::Rejected { reason });
                continue;
            }

            let total_size = self.current_total_size + fragment_size;

            if total_size <= ledger_params.block_content_max_size {
                tracing::debug!("applying fragment in simulation");
                match ledger_simulation.apply_fragment(ledger_params, &fragment, block_date) {
                    Ok(ledger_new) => {
                        self.builder.push(fragment);
                        ledger_simulation = ledger_new;
                        tracing::debug!("successfully applied and committed the fragment");
                    }
                    Err(error) => {
                        let mut msg = error.to_string();
                        for e in iter::successors(error.source(), |&e| e.source()) {
                            msg.push_str(": ");
                            msg.push_str(&e.to_string());
                        }
                        tracing::debug!(?error, "fragment is rejected");
                        logs.modify(id, FragmentStatus::Rejected { reason: msg })
                    }
                }

                self.current_total_size = total_size;

                if total_size == ledger_params.block_content_max_size {
                    break;
                }
            } else {
                // return a fragment to the pool later if does not fit the contents size limit
                return_to_pool.push(fragment);
            }
            drop(_enter);
        }

        pool.insert_all(return_to_pool);
    }
}
