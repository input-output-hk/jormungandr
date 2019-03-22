//! block builder tooling and more
//!

use crate::block::{
    BftProof, Block, BlockContentHash, BlockContents, BlockDate, BlockId, BlockVersion,
    ChainLength, Common, GenesisPraosProof, Header, KESSignature, Message, Proof,
    BLOCK_VERSION_CONSENSUS_BFT, BLOCK_VERSION_CONSENSUS_GENESIS_PRAOS,
    BLOCK_VERSION_CONSENSUS_NONE,
};
use crate::key::{make_signature, make_signature_update};
use crate::leadership::{self, genesis};
use crate::transaction::AuthenticatedTransaction;
use chain_addr::Address;
use chain_crypto::{
    Curve25519_2HashDH, Ed25519Extended, FakeMMM, PublicKey, SecretKey, VerifiableRandomFunction,
};

pub struct BlockBuilder {
    pub common: Common,
    pub contents: BlockContents,
}

impl From<Block> for BlockBuilder {
    fn from(block: Block) -> BlockBuilder {
        BlockBuilder {
            common: block.header.common,
            contents: block.contents,
        }
    }
}

/// block builder to build and finalize the construction of a block
impl BlockBuilder {
    /// default setting, equivalent to writing a genesis block (the empty block)
    pub fn new() -> BlockBuilder {
        use chain_core::property::BlockId;
        BlockBuilder {
            common: Common {
                block_content_size: 0,
                block_content_hash: BlockContentHash::zero(),
                block_version: BLOCK_VERSION_CONSENSUS_NONE,
                block_parent_hash: BlockId::zero(),
                block_date: BlockDate::first(),
                chain_length: ChainLength(0),
            },
            contents: BlockContents::new(Vec::new()),
        }
    }

    /// set the block date
    pub fn date(&mut self, block_date: BlockDate) -> &mut Self {
        self.common.block_date = block_date;
        self
    }

    /// set the chain_length
    pub fn chain_length(&mut self, chain_length: ChainLength) -> &mut Self {
        self.common.chain_length = chain_length;
        self
    }

    /// set the parent hash
    pub fn parent(&mut self, block_parent_hash: BlockId) -> &mut Self {
        self.common.block_parent_hash = block_parent_hash;
        self
    }

    /// set a transaction in the block to build
    ///
    /// Equivalent to call `block_builder.message(Message::Transaction(transaction))`
    pub fn transaction(
        &mut self,
        signed_transaction: AuthenticatedTransaction<Address>,
    ) -> &mut Self {
        self.message(Message::Transaction(signed_transaction))
    }

    /// add a message in the block to build
    pub fn message(&mut self, message: Message) -> &mut Self {
        self.contents.0.push(message);
        self
    }

    /// set multiple messages in the block to build
    pub fn messages<I>(&mut self, messages: I) -> &mut Self
    where
        I: IntoIterator<Item = Message>,
    {
        self.contents.0.extend(messages);
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

    /// create a genesis block (i.e. no signature)
    ///
    /// This is the first ever block of the blockchain and it is expected
    /// the data to be `0.0` and the hash to be `00000000000000...`.
    pub fn make_genesis_block(mut self) -> Block {
        use chain_core::property::BlockId as _;
        assert!(self.common.block_parent_hash == BlockId::zero());
        assert!(self.common.block_date == BlockDate::first());
        assert_eq!(self.common.chain_length, ChainLength(0));
        self.finalize_common(BLOCK_VERSION_CONSENSUS_NONE);
        self.make_block(Proof::None)
    }

    /// create a BFT Block. this block will be signed with the given private key
    pub fn make_bft_block(mut self, bft_signing_key: &SecretKey<Ed25519Extended>) -> Block {
        assert_ne!(self.common.chain_length, ChainLength(0));
        self.finalize_common(BLOCK_VERSION_CONSENSUS_BFT);
        let bft_proof = BftProof {
            leader_id: leadership::bft::LeaderId(bft_signing_key.to_public()),
            signature: super::BftSignature(make_signature(bft_signing_key, &self.common)),
        };
        self.make_block(Proof::Bft(bft_proof))
    }

    /// create a Praos/Genesis block, this block will be signed with the
    /// given KES key.
    pub fn make_genesis_praos_block(
        mut self,
        genesis_praos_id: &genesis::GenesisPraosId,
        kes_signing_key: &mut SecretKey<FakeMMM>,
        vrf_public_key: &PublicKey<Curve25519_2HashDH>,
        vrf_proof: <Curve25519_2HashDH as VerifiableRandomFunction>::VerifiedRandom,
    ) -> Block {
        assert_ne!(self.common.chain_length, ChainLength(0));
        self.finalize_common(BLOCK_VERSION_CONSENSUS_GENESIS_PRAOS);

        let genesis_praos_proof = GenesisPraosProof {
            genesis_praos_id: genesis_praos_id.clone(),
            vrf_proof: vrf_proof,
            // ! SECURITY FIXME ! : also include id and vrf proof.
            kes_proof: KESSignature(make_signature_update(kes_signing_key, &self.common)),
        };
        self.make_block(Proof::GenesisPraos(genesis_praos_proof))
    }
}
