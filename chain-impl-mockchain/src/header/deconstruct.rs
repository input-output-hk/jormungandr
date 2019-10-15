use super::components::{BftSignature, ChainLength, HeaderId, KESSignature, VrfProof};
use super::version::BlockVersion;
use crate::certificate::PoolId;
use crate::date::BlockDate;
use crate::fragment::{BlockContentSize, BlockContentHash};
use crate::leadership;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Common {
    pub block_version: BlockVersion,
    pub block_date: BlockDate,
    pub block_content_size: BlockContentSize,
    pub block_content_hash: BlockContentHash,
    pub block_parent_hash: HeaderId,
    pub chain_length: ChainLength,
}

#[derive(Debug, Clone)]
pub enum Proof {
    /// In case there is no need for consensus layer and no need for proof of the
    /// block. This may apply to the genesis block for example.
    None,
    Bft(BftProof),
    GenesisPraos(GenesisPraosProof),
}

#[derive(Debug, Clone)]
pub struct BftProof {
    pub(crate) leader_id: leadership::bft::LeaderId,
    pub(crate) signature: BftSignature,
}

#[derive(Debug, Clone)]
pub struct GenesisPraosProof {
    pub(crate) node_id: PoolId,
    pub(crate) vrf_proof: VrfProof,
    pub(crate) kes_proof: KESSignature,
}
