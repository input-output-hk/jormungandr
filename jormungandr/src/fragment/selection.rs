use super::logs::Logs;
use super::pool::internal::Pool;
use crate::{
    blockcfg::{BlockDate, Contents, ContentsBuilder, Ledger, LedgerParameters},
    fragment::FragmentId,
};
use chain_core::property::Fragment as _;
use jormungandr_lib::interfaces::FragmentStatus;

use slog::Logger;

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
    logger: Logger,
}

impl OldestFirst {
    pub fn new(logger: Logger) -> Self {
        OldestFirst {
            builder: ContentsBuilder::new(),
            current_total_size: 0,
            logger,
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

            let logger = self.logger.new(o!("hash" => id.to_string()));

            if fragment_size > ledger_params.block_content_max_size {
                let reason = format!(
                    "fragment size {} exceeds maximum block content size {}",
                    fragment_size, ledger_params.block_content_max_size
                );
                debug!(logger, "{}", reason);
                logs.modify(id, FragmentStatus::Rejected { reason }, &logger);
                continue;
            }

            let total_size = self.current_total_size + fragment_size;

            if total_size <= ledger_params.block_content_max_size {
                debug!(logger, "applying fragment in simulation");
                match ledger_simulation.apply_fragment(ledger_params, &fragment, block_date) {
                    Ok(ledger_new) => {
                        self.builder.push(fragment);
                        ledger_simulation = ledger_new;
                        debug!(logger, "successfully applied and committed the fragment");
                    }
                    Err(error) => {
                        use std::error::Error as _;
                        let mut msg = error.to_string();
                        for e in iter::successors(error.source(), |&e| e.source()) {
                            msg.push_str(": ");
                            msg.push_str(&e.to_string());
                        }
                        debug!(logger, "fragment is rejected"; "error" => ?error);
                        logs.modify(id, FragmentStatus::Rejected { reason: msg }, &logger)
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
        }

        pool.insert_all(return_to_pool);
    }
}
