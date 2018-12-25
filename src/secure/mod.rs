pub mod crypto;

use cardano::block;
use cardano::hdwallet;
use cardano::util::hex;
use std::fs;
use std::io;
use std::io::Read;
use std::path::Path;

/// Node Secret(s)
pub struct NodeSecret {
    pub block_privatekey: hdwallet::XPrv,
    pub public: NodePublic,
}

/// Node Secret's Public parts
#[derive(Clone)]
pub struct NodePublic {
    pub block_publickey: hdwallet::XPub,
}

impl NodeSecret {
    pub fn to_public(&self) -> NodePublic {
        self.public.clone()
    }

    pub fn load_from_file(path: &Path) -> io::Result<Self> {
        let mut fs = fs::File::open(path)?;
        let mut vec = Vec::new();
        fs.read_to_end(&mut vec)?;
        let v = hex::decode(String::from_utf8(vec).unwrap().as_ref()).unwrap();
        // TODO propagate error properly
        if v.len() != hdwallet::XPRV_SIZE {
            panic!("wrong size for secret")
        }

        let mut b = [0u8; hdwallet::XPRV_SIZE];
        b.copy_from_slice(&v);
        let prv = hdwallet::XPrv::from_bytes_verified(b).expect("secret key is invalid");
        let np = NodePublic {
            block_publickey: prv.public(),
        };
        Ok(NodeSecret {
            public: np,
            block_privatekey: prv,
        })
    }

    pub fn sign_block(&self) -> block::sign::BlockSignature {
        let _k = &self.block_privatekey;
        let fake_sig = block::sign::BlockSignature::Signature(hdwallet::Signature::from_bytes(
            [0u8; hdwallet::SIGNATURE_SIZE],
        ));
        fake_sig
    }
}
