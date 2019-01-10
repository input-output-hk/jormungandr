//! Module provides cryptographic utilities and types related to
//! the user keys.
//!
use cardano::hash;
use cardano::hdwallet as crypto;
use cardano::util::try_from_slice::TryFromSlice;
use chain_core::property;

// TODO: this public key contains the chain code in it too
// during serialisation this might not be needed
// removing it will save 32bytes of non necessary storage (github #93)

/// Public key of the entity.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct PublicKey(crypto::XPub);
impl AsRef<[u8]> for PublicKey {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}
impl PublicKey {
    pub fn verify(&self, message: &[u8], signature: &Signature) -> bool {
        self.0.verify(message, &signature.0)
    }
}

/// Private key of the entity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrivateKey(crypto::XPrv);
impl AsRef<[u8]> for PrivateKey {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}
impl PrivateKey {
    pub fn public(&self) -> PublicKey {
        PublicKey(self.0.public())
    }
    pub fn sign(&self, data: &[u8]) -> Signature {
        Signature(self.0.sign(data))
    }
}

/// Hash that is used as an address of the various components.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub struct Hash(hash::Blake2b256);
impl Hash {
    pub fn hash_bytes(bytes: &[u8]) -> Self {
        Hash(hash::Blake2b256::new(bytes))
    }
}
impl AsRef<[u8]> for Hash {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl property::BlockId for Hash {
    fn try_from_slice(slice: &[u8]) -> Option<Self> {
        match hash::Blake2b256::try_from_slice(slice) {
            Ok(x) => Some(Hash(x)),
            Err(_) => None,
        }
    }
}

/// Cryptographic signature.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Signature(pub crypto::Signature<()>);
impl AsRef<[u8]> for Signature {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use quickcheck::{Arbitrary, Gen};

    impl Arbitrary for PublicKey {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            let mut xpub = [0; crypto::XPUB_SIZE];
            for byte in xpub.iter_mut() {
                *byte = Arbitrary::arbitrary(g);
            }
            PublicKey(crypto::XPub::from_bytes(xpub))
        }
    }

    impl Arbitrary for PrivateKey {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            let mut xprv = [0; crypto::XPRV_SIZE];
            for byte in xprv.iter_mut() {
                *byte = Arbitrary::arbitrary(g);
            }
            PrivateKey(crypto::XPrv::normalize_bytes(xprv))
        }
    }

    impl Arbitrary for Hash {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            let mut bytes = [0u8; 16];
            for byte in bytes.iter_mut() {
                *byte = Arbitrary::arbitrary(g);
            }
            Hash(hash::Blake2b256::new(&bytes))
        }
    }

}
