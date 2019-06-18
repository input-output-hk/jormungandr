use super::logs::internal::Logs;
use super::pool::internal::Pool;
use crate::{
    blockcfg::{BlockBuilder, HeaderContentEvalContext, Ledger, LedgerParameters},
    fragment::FragmentId,
};
use jormungandr_lib::interfaces::FragmentStatus;

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
        metadata: &HeaderContentEvalContext,
        logs: &mut Logs,
        pool: &mut Pool,
    );

    fn finalize(self) -> BlockBuilder;
}

pub struct OldestFirst {
    builder: BlockBuilder,
    max_per_block: usize,
}

impl OldestFirst {
    pub fn new(max_per_block: usize) -> Self {
        OldestFirst {
            builder: BlockBuilder::new(),
            max_per_block,
        }
    }
}

impl FragmentSelectionAlgorithm for OldestFirst {
    fn finalize(self) -> BlockBuilder {
        self.builder
    }

    fn select(
        &mut self,
        ledger: &Ledger,
        ledger_params: &LedgerParameters,
        metadata: &HeaderContentEvalContext,
        logs: &mut Logs,
        pool: &mut Pool,
    ) {
        let mut total = 0usize;

        while let Some(id) = pool.entries_by_time.pop_front() {
            if total >= self.max_per_block {
                break;
            }

            let fragment = pool.remove(&id).unwrap();

            match ledger.apply_fragment(ledger_params, &fragment, metadata) {
                Ok(_) => {
                    self.builder.message(fragment);

                    logs.modify(
                        &id.into(),
                        FragmentStatus::InABlock {
                            date: metadata.block_date.into(),
                        },
                    );

                    total += 1;
                }
                Err(error) => {
                    use std::error::Error as _;
                    let error = if let Some(source) = error.source() {
                        format!("{}: {}", error, source)
                    } else {
                        error.to_string()
                    };
                    logs.modify(&id.into(), FragmentStatus::Rejected { reason: error })
                }
            }
        }
    }
}
