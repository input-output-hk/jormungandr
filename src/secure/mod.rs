pub mod crypto;

use cardano::util::hex;
use chain_crypto::{Ed25519Extended, PublicKey, SecretKey};
use cryptoxide::ed25519;
use std::fs;
use std::io;
use std::io::Read;
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

    pub fn load_from_file(path: &Path) -> io::Result<Self> {
        let mut fs = fs::File::open(path)?;
        let mut vec = Vec::new();
        fs.read_to_end(&mut vec)?;
        let v = hex::decode(String::from_utf8(vec).unwrap().as_ref()).unwrap();
        // TODO propagate error properly
        if v.len() != ed25519::PRIVATE_KEY_LENGTH {
            panic!("wrong size for secret")
        }
        let prv = SecretKey::from_bytes(&v).unwrap();
        let np = NodePublic {
            block_publickey: prv.to_public(),
        };
        Ok(NodeSecret {
            public: np,
            block_privatekey: prv,
        })
    }
}
