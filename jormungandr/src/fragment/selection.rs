use super::logs::Logs;
use super::pool::internal::Pool;
use crate::{
    blockcfg::{BlockDate, Contents, ContentsBuilder, Ledger, LedgerParameters},
    fragment::FragmentId,
};
use chain_core::property::Fragment as _;
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

        while let Some(fragment) = pool.remove_oldest() {
            let id = fragment.id();
            let fragment_raw = fragment.to_raw(); // TODO: replace everything to FragmentRaw in the node
            let fragment_size = fragment_raw.size_bytes_plus_size() as u32;
            let total_size = self.current_total_size + fragment_size;

            if total_size <= ledger_params.block_content_max_size {
                match ledger_simulation.apply_fragment(ledger_params, &fragment, block_date) {
                    Ok(ledger_new) => {
                        self.builder.push(fragment);
                        ledger_simulation = ledger_new;
                    }
                    Err(error) => {
                        use std::error::Error as _;
                        let error = if let Some(source) = error.source() {
                            format!("{}: {}", error, source)
                        } else {
                            error.to_string()
                        };
                        logs.modify(id, FragmentStatus::Rejected { reason: error })
                    }
                }

                self.current_total_size = total_size;

                if total_size == ledger_params.block_content_max_size {
                    break;
                }
            }
        }
    }
}
