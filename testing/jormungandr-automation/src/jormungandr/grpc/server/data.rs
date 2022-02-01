use super::ProtocolVersion;
use chain_core::{
    mempack::{ReadBuf, ReadError, Readable},
    property::Serialize,
};
use chain_crypto::{Ed25519, KeyPair, PublicKey, Signature, Verification};
use chain_impl_mockchain::{
    block::{self, Block, BlockDate, ContentsBuilder, Header},
    header::BlockVersion,
    key::Hash,
};
use chain_storage::{BlockInfo, BlockStore};
use rand::Rng;
use std::net::SocketAddr;

const AUTH_NONCE_LEN: usize = 32;

pub struct MockServerData {
    genesis_hash: Hash,
    protocol: ProtocolVersion,
    keypair: KeyPair<Ed25519>,
    profile: poldercast::Profile,
    auth_nonce: [u8; AUTH_NONCE_LEN],
    storage: BlockStore,
    invalid_block0_hash: bool,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Storage(#[from] chain_storage::Error),
    #[error(transparent)]
    Read(#[from] ReadError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

impl MockServerData {
    const TIP_TAG: &'static str = "tip";
    pub fn new(
        genesis_hash: Hash,
        protocol: ProtocolVersion,
        addr: SocketAddr,
        storage: BlockStore,
        invalid_get_blocks_hash: bool,
    ) -> Self {
        let keypair = KeyPair::generate(&mut rand::thread_rng());
        let topology_key = keynesis::key::ed25519::SecretKey::new(&mut rand::thread_rng());
        let profile = poldercast::Profile::new(addr, &topology_key);
        Self {
            genesis_hash,
            protocol,
            keypair,
            profile,
            auth_nonce: [0; AUTH_NONCE_LEN],
            storage,
            invalid_block0_hash: invalid_get_blocks_hash,
        }
    }

    pub fn genesis_hash(&self) -> &Hash {
        &self.genesis_hash
    }

    pub fn get_block(&self, header_id: Hash) -> Result<Block, Error> {
        Ok(Block::read(&mut ReadBuf::from(
            self.storage()
                .get_block(header_id.as_ref())
                .unwrap()
                .as_ref(),
        ))?)
    }

    pub fn genesis_block(&self) -> Block {
        self.get_block(self.genesis_hash)
            .expect("genesis block should always be valid")
    }

    pub fn tip(&self) -> Result<Header, Error> {
        let header_id = self
            .storage
            .get_tag(Self::TIP_TAG)?
            .ok_or(chain_storage::Error::BlockNotFound)?;

        Ok(self
            .get_block(Hash::read(&mut ReadBuf::from(header_id.as_ref())).unwrap())?
            .header()
            .clone())
    }

    /// block_id must refer to a valid block already in the storage
    pub fn set_tip(&self, block_id: &[u8]) -> Result<(), Error> {
        Ok(self.storage.put_tag(Self::TIP_TAG, block_id)?)
    }

    pub fn put_block(&self, block: &Block) -> Result<(), Error> {
        let id = block.header().hash().serialize_as_vec()?;
        let parent_id = block.header().block_parent_hash().serialize_as_vec()?;
        let chain_length = block.header().chain_length().into();
        let block_info = BlockInfo::new(id, parent_id, chain_length);
        Ok(self
            .storage()
            .put_block(&block.serialize_as_vec()?, block_info)?)
    }

    pub fn profile(&self) -> &poldercast::Profile {
        &self.profile
    }

    pub fn protocol(&self) -> &ProtocolVersion {
        &self.protocol
    }

    pub fn node_id(&self) -> &[u8] {
        self.keypair.public_key().as_ref()
    }

    pub fn node_signature(&self, nonce: &[u8]) -> Vec<u8> {
        let signature = self.keypair.private_key().sign(nonce);
        signature.as_ref().to_vec()
    }

    pub fn generate_auth_nonce(&mut self) -> &[u8] {
        rand::thread_rng().fill(&mut self.auth_nonce[..]);
        &self.auth_nonce
    }

    /// Raw access to the storage. Can be used to play with raw bytes sequences
    /// or get around validations of domain objects, for example to mimic a malicous
    /// adversary
    pub fn storage(&self) -> &BlockStore {
        &self.storage
    }

    pub fn validate_peer_node_id(&self, node_id: &[u8], signature: &[u8]) -> bool {
        let public_key = PublicKey::<Ed25519>::from_binary(node_id).expect("invalid node ID");
        let signature =
            Signature::<[u8], Ed25519>::from_binary(signature).expect("invalid signature");
        match signature.verify(&public_key, &self.auth_nonce) {
            Verification::Success => true,
            Verification::Failed => false,
        }
    }

    pub fn genesis_hash_mut(&mut self) -> &mut Hash {
        &mut self.genesis_hash
    }

    pub fn protocol_mut(&mut self) -> &mut ProtocolVersion {
        &mut self.protocol
    }

    pub fn invalid_block0_hash(&self) -> bool {
        self.invalid_block0_hash
    }
}

pub fn block0() -> Block {
    block::builder(
        BlockVersion::Genesis,
        ContentsBuilder::new().into(),
        |hdr| {
            Ok::<_, ()>(
                hdr.set_genesis()
                    .set_date(BlockDate::first())
                    .into_unsigned_header()
                    .expect("internal error cannot build unsigned block")
                    .generalize(),
            )
        },
    )
    .expect("internal error: block builder cannot return error")
}
