//! block builder tooling and more
//!

use crate::block::{
    BftProof, Block, BlockContentHash, BlockContents, BlockDate, BlockId, BlockVersion, Common,
    GenesisPraosProof, Header, KESSignature, Message, Proof, BLOCK_VERSION_CONSENSUS_BFT,
    BLOCK_VERSION_CONSENSUS_GENESIS_PRAOS, BLOCK_VERSION_CONSENSUS_NONE,
};
use crate::key::{make_signature, make_signature_update};
use crate::transaction::SignedTransaction;
use chain_crypto::{vrf::vrf, Ed25519Extended, FakeMMM, SecretKey};

pub struct BlockBuilder {
    pub common: Common,
    pub contents: BlockContents,
}

impl BlockBuilder {
    pub fn new() -> BlockBuilder {
        use chain_core::property::BlockId;
        BlockBuilder {
            common: Common {
                block_content_size: 0,
                block_content_hash: BlockContentHash::zero(),
                block_version: BLOCK_VERSION_CONSENSUS_NONE,
                block_parent_hash: BlockId::zero(),
                block_date: BlockDate::first(),
            },
            contents: BlockContents::new(Vec::new()),
        }
    }

    pub fn date(&mut self, block_date: BlockDate) -> &mut Self {
        self.common.block_date = block_date;
        self
    }

    pub fn parent(&mut self, block_parent_hash: BlockId) -> &mut Self {
        self.common.block_parent_hash = block_parent_hash;
        self
    }

    pub fn transactions(&mut self, signed_transaction: SignedTransaction) -> &mut Self {
        self.contents
            .0
            .push(Message::Transaction(signed_transaction));
        self
    }

    pub fn message(&mut self, message: Message) -> &mut Self {
        self.contents.0.push(message);
        self
    }

    fn make_block(self, proof: Proof) -> Block {
        Block {
            header: Header {
                common: self.common,
                proof: proof,
            },
            contents: self.contents,
        }
    }

    fn finalize_common(&mut self, block_version: BlockVersion) -> &mut Self {
        let (content_hash, content_size) = self.contents.compute_hash_size();
        self.common.block_content_hash = content_hash;
        self.common.block_content_size = content_size as u32;
        self.common.block_version = block_version;
        self
    }

    pub fn make_genesis_block(mut self) -> Block {
        self.finalize_common(BLOCK_VERSION_CONSENSUS_NONE);
        self.make_block(Proof::None)
    }

    pub fn make_bft_block(mut self, bft_signing_key: &SecretKey<Ed25519Extended>) -> Block {
        self.finalize_common(BLOCK_VERSION_CONSENSUS_BFT);
        let bft_proof = BftProof {
            leader_id: bft_signing_key.to_public().into(),
            signature: super::BftSignature(make_signature(bft_signing_key, &self.common)),
        };
        self.make_block(Proof::Bft(bft_proof))
    }

    pub fn make_genesis_praos_block(
        mut self,
        kes_signing_key: &mut SecretKey<FakeMMM>,
        vrf_public_key: vrf::PublicKey,
        vrf_proof: vrf::ProvenOutputSeed,
    ) -> Block {
        self.finalize_common(BLOCK_VERSION_CONSENSUS_GENESIS_PRAOS);

        let genesis_praos_proof = GenesisPraosProof {
            kes_proof: KESSignature(make_signature_update(kes_signing_key, &self.common)),
            kes_public_key: kes_signing_key.to_public(),
            vrf_proof: vrf_proof,
            vrf_public_key: vrf_public_key,
        };
        self.make_block(Proof::GenesisPraos(genesis_praos_proof))
    }
}
