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
    inner: Arc<SecureEnclave>,
}

impl Enclave {
    /// create a new enclave structure. This will need to change in the future
    /// with a parameter to contact the secure enclave instead of the dummy type.
    pub fn new(secure_enclave: SecureEnclave) -> Self {
        Enclave {
            inner: Arc::new(secure_enclave),
        }
    }

    /// ask the enclave to attempt computing some leadership schedule for the
    /// given settings
    ///
    /// TODO: for now we are utilizing the Leadership object fully but on the long
    ///       run this might be limited to only the required data.
    pub async fn query_schedules(
        &self,
        leadership: Arc<Leadership>,
        slot_start: u32,
        nb_slots: u32,
    ) -> Result<Vec<LeaderEvent>, EnclaveError> {
        Ok(self
            .inner
            .leadership_evaluate(&leadership, slot_start, nb_slots)
            .await)
    }

    /// ask the leader associated to the `LeaderEvent` to finalize the given
    /// block by providing the proof.
    ///
    /// TODO: for now we are querying the whole with the block builder but on the long
    ///       run we will only need the block signing data.
    pub async fn query_header_bft_finalize(
        &self,
        block_builder: HeaderBftBuilder<HeaderSetConsensusSignature>,
        id: LeaderId,
    ) -> Result<HeaderBft, EnclaveError> {
        if let Some(block) = self.inner.create_header_bft(block_builder, id).await {
            Ok(block)
        } else {
            Err(EnclaveError::NotInEnclave { id })
        }
    }

    /// ask the leader associated to the `LeaderEvent` to finalize the given
    /// block by providing the proof.
    ///
    /// TODO: for now we are querying the whole with the block builder but on the long
    ///       run we will only need the block signing data.
    pub async fn query_header_genesis_praos_finalize(
        &self,
        block_builder: HeaderGenesisPraosBuilder<HeaderSetConsensusSignature>,
        id: LeaderId,
    ) -> Result<HeaderGenesisPraos, EnclaveError> {
        if let Some(block) = self
            .inner
            .create_header_genesis_praos(block_builder, id)
            .await
        {
            Ok(block)
        } else {
            Err(EnclaveError::NotInEnclave { id })
        }
    }
}
