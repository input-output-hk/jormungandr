use super::components::{ChainLength, HeaderId, VrfProof};
use super::cstruct;
use super::header::{HeaderBft, HeaderGenesisPraos, HeaderUnsigned};
use super::version::BlockVersion;

use crate::block::{BftSignature, KESSignature};
use crate::certificate::PoolId;
use crate::date::BlockDate;
use crate::fragment::{BlockContentHash, BlockContentSize, Contents};
use crate::leadership;

use chain_crypto::{Ed25519, SecretKey, SumEd25519_12};
use std::marker::PhantomData;

/// Finalized BFT Header
pub struct HeaderBuilder<HeaderBuildingState: ?Sized>(
    cstruct::Header,
    PhantomData<HeaderBuildingState>,
);

/// Header Builder (finalize BFT part)
pub struct HeaderBftBuilder<BftBuildingState: ?Sized>(
    cstruct::Header,
    PhantomData<BftBuildingState>,
);

/// Header Builder (finalize Genesis-Praos part)
pub struct HeaderGenesisPraosBuilder<GpBuildingState: ?Sized>(
    cstruct::Header,
    PhantomData<GpBuildingState>,
);

// state machine
pub enum HeaderSetDate {}
pub enum HeaderSetParenting {}
pub enum HeaderCommonDone {}
pub enum HeaderSetConsensusData {}
pub enum HeaderSetConsensusSignature {}
// end

fn header_builder_raw(
    version: BlockVersion,
    content_hash: &BlockContentHash,
    content_size: BlockContentSize,
) -> HeaderBuilder<HeaderSetParenting> {
    let mut hdr = cstruct::Header::new(version.to_u16());
    hdr.set_content_size(content_size);
    hdr.set_content_hash(content_hash.into());
    HeaderBuilder(hdr, PhantomData)
}

pub fn header_builder(
    version: BlockVersion,
    contents: &Contents,
) -> HeaderBuilder<HeaderSetParenting> {
    let block_content_info = contents.compute_hash_size();
    header_builder_raw(version, &block_content_info.0, block_content_info.1)
}

pub type HeaderBuilderNew = HeaderBuilder<HeaderSetParenting>;

impl HeaderBuilderNew {
    /// Create a new Header builder starting from the full content.
    ///
    /// This doesn't need the content directly, but only uses the content to calculate
    /// the content hash and the content size, and make sure this is consistent
    pub fn new(version: BlockVersion, contents: &Contents) -> Self {
        header_builder(version, contents)
    }

    /// recommended to use new(), this is only for test
    pub fn new_raw(
        version: BlockVersion,
        content_hash: &BlockContentHash,
        content_size: BlockContentSize,
    ) -> Self {
        header_builder_raw(version, content_hash, content_size)
    }
}

impl HeaderBuilder<HeaderSetParenting> {
    /// Set the header as a genesis header:
    /// * the depth starts at 0
    /// * the parent is set to the "null hash" (hash all 0)
    pub fn set_genesis(self) -> HeaderBuilder<HeaderSetDate> {
        let mut hdr = self.0;
        hdr.set_height(0);
        hdr.set_parent_hash(&HeaderId::zero_hash().into());
        HeaderBuilder(hdr, PhantomData)
    }

    /// Set the header as a general block, with a specific depth
    /// and parent hash
    pub fn set_parent(
        self,
        parent_hash: &HeaderId,
        chain_length: ChainLength,
    ) -> HeaderBuilder<HeaderSetDate> {
        let mut hdr = self.0;
        hdr.set_height(chain_length.0);
        hdr.set_parent_hash(&parent_hash.clone().into());
        HeaderBuilder(hdr, PhantomData)
    }
}

impl HeaderBuilder<HeaderSetDate> {
    /// Set the date of this block
    pub fn set_date(self, date: BlockDate) -> HeaderBuilder<HeaderCommonDone> {
        let mut hdr = self.0;
        hdr.set_date_epoch(date.epoch);
        hdr.set_date_slotid(date.slot_id);
        HeaderBuilder(hdr, PhantomData)
    }
}

impl HeaderBuilder<HeaderCommonDone> {
    /// Finalized to an unsigned header
    pub fn to_unsigned_header(self) -> Option<HeaderUnsigned> {
        match self.0.version() {
            cstruct::VERSION_UNSIGNED => Some(HeaderUnsigned(self.0)),
            _ => None,
        }
    }

    /// Tentatively transition to a BFT Header builder
    pub fn to_bft_builder(self) -> Option<HeaderBftBuilder<HeaderSetConsensusData>> {
        match self.0.version() {
            cstruct::VERSION_BFT => Some(HeaderBftBuilder(self.0, PhantomData)),
            _ => None,
        }
    }

    /// Tentatively transition to a Genesis-Praos Header builder
    pub fn to_genesis_praos_builder(
        self,
    ) -> Option<HeaderGenesisPraosBuilder<HeaderSetConsensusData>> {
        match self.0.version() {
            cstruct::VERSION_GP => Some(HeaderGenesisPraosBuilder(self.0, PhantomData)),
            _ => None,
        }
    }
}

impl HeaderBftBuilder<HeaderSetConsensusData> {
    pub fn sign_using(self, sk: &SecretKey<Ed25519>) -> HeaderBft {
        let pk = sk.to_public();
        let sret = self.set_consensus_data(&leadership::bft::LeaderId(pk));
        let sig = sk.sign_slice(sret.get_authenticated_data());

        sret.set_signature(BftSignature(sig))
    }

    pub fn set_consensus_data(
        self,
        bft_leaderid: &leadership::bft::LeaderId,
    ) -> HeaderBftBuilder<HeaderSetConsensusSignature> {
        let mut hdr = self.0;
        hdr.set_bft_leader_id_slice(bft_leaderid.0.as_ref());
        HeaderBftBuilder(hdr, PhantomData)
    }
}

impl HeaderGenesisPraosBuilder<HeaderSetConsensusData> {
    pub fn set_consensus_data(
        self,
        node_id: &PoolId,
        vrf_proof: &VrfProof,
    ) -> HeaderGenesisPraosBuilder<HeaderSetConsensusSignature> {
        let mut hdr = self.0;
        hdr.set_gp_node_id(node_id.into());
        hdr.set_gp_vrf_proof(&vrf_proof.0);
        HeaderGenesisPraosBuilder(hdr, PhantomData)
    }
}

impl HeaderBftBuilder<HeaderSetConsensusSignature> {
    /// Get the authenticated data of a BFT header being built
    ///
    /// Typically this is used to generate the signature
    pub fn get_authenticated_data(&self) -> &[u8] {
        self.0.as_slice().slice_bft_auth()
    }

    /// Set the signature in the BFT header and return the finalized BFT header
    pub fn set_signature(self, bft_signature: BftSignature) -> HeaderBft {
        let mut hdr = self.0;
        hdr.set_bft_signature_slice(bft_signature.0.as_ref());
        HeaderBft(hdr)
    }
}

impl HeaderGenesisPraosBuilder<HeaderSetConsensusSignature> {
    pub fn get_authenticated_data<'a>(&'a self) -> &'a [u8] {
        self.0.as_slice().slice_gp_auth()
    }

    /// Set the signature in the Genesis-Praos header and return a finalized Genesis-Praos Header
    pub fn set_signature(self, kes_signature: KESSignature) -> HeaderGenesisPraos {
        let mut hdr = self.0;
        hdr.set_gp_kes_signature_slice(kes_signature.0.as_ref());
        HeaderGenesisPraos(hdr)
    }

    /// Just a helper to set the signature directly from what the secret key generate
    pub fn sign_using(self, kes_signing_key: &SecretKey<SumEd25519_12>) -> HeaderGenesisPraos {
        let data = self.get_authenticated_data();
        let signature = kes_signing_key.sign_slice(data);
        self.set_signature(KESSignature(signature))
    }
}
