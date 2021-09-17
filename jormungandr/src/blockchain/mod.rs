mod branch;
mod candidate;
mod chain;
mod chain_selection;
mod checkpoints;
mod multiverse;
mod process;
mod reference;
mod reference_cache;
mod storage;
mod tip;

// Constants

mod chunk_sizes {
    // The maximum number of blocks to request per each GetBlocks request
    // or a Solicit event when pulling missing blocks.
    //
    // This may need to be made into a configuration parameter.
    // The number used here aims for this number of block IDs to fit within
    // a reasonable network path MTU, leaving room for gRPC and TCP/IP framing.
    pub const BLOCKS: u64 = 32;
}

// Re-exports

pub use self::{
    branch::Branch,
    chain::{
        new_epoch_leadership_from, Blockchain, CheckHeaderProof, EpochLeadership, Error,
        LeadershipBlock, PreCheckedHeader, MAIN_BRANCH_TAG,
    },
    chain_selection::{compare_against, ComparisonResult},
    checkpoints::Checkpoints,
    multiverse::Multiverse,
    process::{start, TaskData},
    reference::Ref,
    storage::{Error as StorageError, Storage},
    tip::{Tip, TipUpdater}, // TODO: Remove TipUpdater as soon as the bootstrap process is refactored
};
