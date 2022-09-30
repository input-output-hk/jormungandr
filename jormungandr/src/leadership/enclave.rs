pub use crate::secure::enclave::{LeaderEvent, Schedule};
use crate::{
    blockcfg::{
        HeaderBft, HeaderBftBuilder, HeaderGenesisPraos, HeaderGenesisPraosBuilder,
        HeaderSetConsensusSignature, Leadership,
    },
    secure::enclave::Enclave as SecureEnclave,
};
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum EnclaveError {
    #[error("Enclave does not have a leader set")]
    EmptyEnclave,
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
    ) -> Result<Schedule, EnclaveError> {
        Ok(Schedule::new(
            self.inner.clone(),
            leadership,
            slot_start,
            nb_slots,
        ))
    }

    /// ask the leader associated to the `LeaderEvent` to finalize the given
    /// block by providing the proof.
    ///
    /// TODO: for now we are querying the whole with the block builder but on the long
    ///       run we will only need the block signing data.
    pub fn query_header_bft_finalize(
        &self,
        block_builder: HeaderBftBuilder<HeaderSetConsensusSignature>,
    ) -> Result<HeaderBft, EnclaveError> {
        if let Some(block) = self.inner.create_header_bft(block_builder) {
            Ok(block)
        } else {
            Err(EnclaveError::EmptyEnclave)
        }
    }

    /// ask the leader associated to the `LeaderEvent` to finalize the given
    /// block by providing the proof.
    ///
    /// TODO: for now we are querying the whole with the block builder but on the long
    ///       run we will only need the block signing data.
    pub fn query_header_genesis_praos_finalize(
        &self,
        block_builder: HeaderGenesisPraosBuilder<HeaderSetConsensusSignature>,
    ) -> Result<HeaderGenesisPraos, EnclaveError> {
        if let Some(block) = self.inner.create_header_genesis_praos(block_builder) {
            Ok(block)
        } else {
            Err(EnclaveError::EmptyEnclave)
        }
    }
}
