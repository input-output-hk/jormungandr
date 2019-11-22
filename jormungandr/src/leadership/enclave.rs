pub use crate::secure::enclave::LeaderEvent;
use crate::{
    blockcfg::{
        HeaderBft, HeaderBftBuilder, HeaderGenesisPraos, HeaderGenesisPraosBuilder,
        HeaderSetConsensusSignature, Leadership,
    },
    secure::enclave::Enclave as SecureEnclave,
};
use jormungandr_lib::interfaces::EnclaveLeaderId as LeaderId;
use std::sync::Arc;
use thiserror::Error;
use tokio::{prelude::*, sync::lock::Lock};

#[derive(Debug, Clone, Error)]
pub enum EnclaveError {
    #[error("This leader {id} is not in the enclave")]
    NotInEnclave { id: LeaderId },
}

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
    /// TODO: for now we are utilizing the Leadership object fully but on the long
    ///       run this might be limited to only the required data.
    pub fn query_schedules(
        &self,
        leadership: Arc<Leadership>,
        slot_start: u32,
        nb_slots: u32,
    ) -> impl Future<Item = Vec<LeaderEvent>, Error = EnclaveError> {
        let mut inner = self.inner.clone();
        future::poll_fn(move || Ok(inner.poll_lock()))
            .map(move |guard| guard.leadership_evaluate(&leadership, slot_start, nb_slots))
    }

    /// ask the leader associated to the `LeaderEvent` to finalize the given
    /// block by providing the proof.
    ///
    /// TODO: for now we are querying the whole with the block builder but on the long
    ///       run we will only need the block signing data.
    pub fn query_header_bft_finalize(
        &self,
        block_builder: HeaderBftBuilder<HeaderSetConsensusSignature>,
        id: LeaderId,
    ) -> impl Future<Item = HeaderBft, Error = EnclaveError> {
        let mut inner = self.inner.clone();

        future::poll_fn(move || Ok(inner.poll_lock())).and_then(move |guard| {
            if let Some(block) = guard.create_header_bft(block_builder, id) {
                future::ok(block)
            } else {
                future::err(EnclaveError::NotInEnclave { id })
            }
        })
    }

    /// ask the leader associated to the `LeaderEvent` to finalize the given
    /// block by providing the proof.
    ///
    /// TODO: for now we are querying the whole with the block builder but on the long
    ///       run we will only need the block signing data.
    pub fn query_header_genesis_praos_finalize(
        &self,
        block_builder: HeaderGenesisPraosBuilder<HeaderSetConsensusSignature>,
        id: LeaderId,
    ) -> impl Future<Item = HeaderGenesisPraos, Error = EnclaveError> {
        let mut inner = self.inner.clone();

        future::poll_fn(move || Ok(inner.poll_lock())).and_then(move |guard| {
            if let Some(block) = guard.create_header_genesis_praos(block_builder, id) {
                future::ok(block)
            } else {
                future::err(EnclaveError::NotInEnclave { id })
            }
        })
    }
}
