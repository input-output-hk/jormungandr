use super::ProtocolVersion;
use chain_crypto::{Ed25519, KeyPair, PublicKey, Signature, Verification};
use chain_impl_mockchain::{
    block::{BlockDate, Header},
    key::Hash,
    testing::{GenesisPraosBlockBuilder, StakePoolBuilder},
};
use chain_time::{Epoch, TimeEra};
use rand::Rng;
use std::net::SocketAddr;

const AUTH_NONCE_LEN: usize = 32;

pub struct MockServerData {
    genesis_hash: Hash,
    tip: Header,
    protocol: ProtocolVersion,
    keypair: KeyPair<Ed25519>,
    profile: poldercast::Profile,
    auth_nonce: [u8; AUTH_NONCE_LEN],
}

impl MockServerData {
    pub fn new(
        genesis_hash: Hash,
        tip: Header,
        protocol: ProtocolVersion,
        addr: SocketAddr,
    ) -> Self {
        let keypair = KeyPair::generate(&mut rand::thread_rng());
        let topology_key = keynesis::key::ed25519::SecretKey::new(&mut rand::thread_rng());
        let profile = poldercast::Profile::new(addr, &topology_key);
        Self {
            genesis_hash,
            tip,
            protocol,
            keypair,
            profile,
            auth_nonce: [0; AUTH_NONCE_LEN],
        }
    }

    pub fn genesis_hash(&self) -> &Hash {
        &self.genesis_hash
    }

    pub fn tip(&self) -> &Header {
        &self.tip
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

    pub fn tip_mut(&mut self) -> &mut Header {
        &mut self.tip
    }

    pub fn protocol_mut(&mut self) -> &mut ProtocolVersion {
        &mut self.protocol
    }
}

pub fn header(slots_per_epochs: u32, parent_id: &Hash) -> Header {
    let stake_pool = StakePoolBuilder::new().build();

    let time_era = TimeEra::new(0u64.into(), Epoch(0u32), slots_per_epochs);

    let block = GenesisPraosBlockBuilder::new()
        .with_parent_id(*parent_id)
        .with_date(BlockDate {
            epoch: 0,
            slot_id: 1,
        })
        .with_chain_length(1.into())
        .build(&stake_pool, &time_era);
    block.header().clone()
}
