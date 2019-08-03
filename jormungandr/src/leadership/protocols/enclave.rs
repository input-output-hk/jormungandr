use crate::{
    blockcfg::{Block, BlockBuilder, Leadership},
    secure::enclave::Enclave as SecureEnclave,
};
use std::sync::Arc;
use tokio::{prelude::*, sync::lock::Lock};

pub use crate::secure::enclave::{LeaderEvent, LeaderId as EnclaveLeaderId};

error_chain! {}

/// represent the client side of an enclave. From there we will query the
/// actual enclave about schedules and signing blocks
///
#[derive(Clone)]
pub struct Enclave {
    /// TODO: we will need to remove this to instead query data to an
    /// external service (running in the secure enclave). For now we
    /// hold on the `SecureEnclave` . But it will need to be separated
    /// when we have the necessary crypto done.
    inner: Lock<SecureEnclave>,
}

impl Enclave {
    /// create a new enclave structure. This will need to change in the future
    /// with a parameter to contact the secure enclave instead of the dummy type.
    pub fn new(secure_enclave: SecureEnclave) -> Self {
        Enclave {
            inner: Lock::new(secure_enclave),
        }
    }

    /// ask the enclave to attempt computing some leadership schedule for the
    /// given settings
    ///
    /// TODO: for now we are utilising the Leadership object fully but on the long
    ///       run this might be limited to only the required data.
    pub fn query_schedules(
        &self,
        leadership: Arc<Leadership>,
        slot_start: u32,
        nb_slots: u32,
    ) -> impl Future<Item = Vec<LeaderEvent>, Error = Error> {
        let mut inner = self.inner.clone();
        future::poll_fn(move || Ok(inner.poll_lock()))
            .map(move |guard| guard.leadership_evaluate(&leadership, slot_start, nb_slots))
    }

    /// ask the leader associated to the `LeaderEvent` to finalize the given
    /// block by providing the proof.
    ///
    /// TODO: for now we are querying the whole with the block builder but on the long
    ///       run we will only need the block signing data.
    pub fn query_block_finalize(
        &self,
        block_builder: BlockBuilder,
        event: LeaderEvent,
    ) -> impl Future<Item = Block, Error = Error> {
        let mut inner = self.inner.clone();

        future::poll_fn(move || Ok(inner.poll_lock())).and_then(move |guard| {
            if let Some(block) = guard.create_block(block_builder, event) {
                future::ok(block)
            } else {
                future::err("Leader is not in the enclave to sign the block".into())
            }
        })
    }
}
