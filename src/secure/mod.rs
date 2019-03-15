pub mod crypto;

use bech32::{Bech32, FromBase32};
use chain_crypto::{AsymmetricKey, Ed25519Extended, PublicKey, SecretKey};
use std::fs;
use std::path::Path;

/// Node Secret(s)
pub struct NodeSecret {
    pub block_privatekey: SecretKey<Ed25519Extended>,
    pub public: NodePublic,
}

/// Node Secret's Public parts
#[derive(Clone)]
pub struct NodePublic {
    pub block_publickey: PublicKey<Ed25519Extended>,
}

impl NodeSecret {
    pub fn public(&self) -> NodePublic {
        self.public.clone()
    }

    pub fn load_from_file(path: &Path) -> NodeSecret {
        let file_string = fs::read_to_string(path).unwrap();
        let bech32: Bech32 = file_string
            .parse()
            .expect("Private key file should be bech32 encoded");
        if bech32.hrp() != Ed25519Extended::SECRET_BECH32_HRP {
            panic!("Private key file should contain Ed25519 extended private key")
        }
        let bytes = Vec::<u8>::from_base32(bech32.data()).unwrap();
        let block_privatekey = SecretKey::from_bytes(&bytes).unwrap();
        let block_publickey = block_privatekey.to_public();
        NodeSecret {
            public: NodePublic { block_publickey },
            block_privatekey,
        }
    }
}
