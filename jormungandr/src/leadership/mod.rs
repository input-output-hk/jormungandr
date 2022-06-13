//! new module to prepare for the new leadership scheduling of blocks
//!
//! here we need to take into consideration that won't have access to the
//! cryptographic objects of the leader: they will be executed in a secure
//! enclave.
//!
//! ## data structures
//!
//! We need to separate our components as following:
//!
//! 1. the enclave:
//!     * upon receiving the necessary parameters, it will return a schedule
//!       when it should be elected to create a block;
//!     * upon receiving the necessary parameters, it will create the
//!       proof to finalize the creation of a block;
//! 2. the schedule:
//!     * it holds the schedules for a given epoch
//!     * we can query it to get a list of schedule for the REST API (useful to
//!       have information when the node is expected to create blocks);
//!     * optional but useful: have a way to update if a schedule has been
//!       executed (and what time);
//!     * optional: have a way for the blockchain task to update
//!       the schedule to know if the scheduled block as been accepted in the
//!       branch;
//!
//! The enclave is not yet implemented, but we will need to separate the crypto
//! from the representation here.
//!
//! ## workflow
//!
//! The flow process will work as follow:
//!
//! 1. the leadership module will receive a new event to create prepare a leadership
//!    schedule; It will only includes the `Leadership` object from chain_lib and the
//!    `TimeFrame` active for the future blocks to come;
//! 2. upon receiving these data, it will query the **enclave** to know the list of expected
//!    scheduled leader elections; (this part may require heavy cryptographic computation,
//!    we may want to split this part into incrementally long queries);
//! 3. once the schedule is retrieved (even partially) we can start waiting for the appropriate
//!    time to create a new block (to run block fragment selection) and ask the enclave to sign
//!    the block;
//! 4. once a block is ready we need to send it to the blockchain task to process it and update
//!    the blockchain.
//!
//! ## how and when to trigger a new leadership event
//!
//! The blockchain module has the material to create the new leadership parameters
//! for a given epoch (the `Leadership` object and the time frame). It needs to send
//! the appropriate data when necessary.
//!
//! 2 ways to trigger a new leadership schedule from the blockchain module:
//!
//! 1. the blockchain detects an epoch transition,
//! 2. the leadership sent an end of epoch signal to the blockchain;
//!
//! Now doing so we may trigger the same leader schedule twice. We will need to make sure
//! we don't duplicate the work everywhere.
//!

mod enclave;
mod logs;
mod process;

pub use self::{
    enclave::{Enclave, EnclaveError, LeaderEvent},
    logs::{LeadershipLogHandle, Logs},
    process::{Module, ModuleConfig},
};
