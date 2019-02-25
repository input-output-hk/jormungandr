pub mod crypto;

use cardano::block;
use cardano::redeem;
use cardano::util::hex;
use std::fs;
use std::io;
use std::io::Read;
use std::path::Path;

/// Node Secret(s)
pub struct NodeSecret {
    pub block_privatekey: redeem::PrivateKey,
    pub public: NodePublic,
}

/// Node Secret's Public parts
#[derive(Clone)]
pub struct NodePublic {
    pub block_publickey: redeem::PublicKey,
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
        if v.len() != redeem::PRIVATEKEY_SIZE {
            panic!("wrong size for secret")
        }

        let mut b = [0u8; redeem::PRIVATEKEY_SIZE];
        b.copy_from_slice(&v);
        let prv = redeem::PrivateKey::normalize_bytes(b);
        let np = NodePublic {
            block_publickey: prv.public(),
        };
        Ok(NodeSecret {
            public: np,
            block_privatekey: prv,
        })
    }
}
