//! Abstraction over addresses in the mockchain.
use crate::key::*;

/// Address. Currently address is just a hash of
/// the public key.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub struct Address(Hash);
impl Address {
    pub fn new(public_key: &PublicKey) -> Self {
        Address(Hash::hash_bytes(public_key.as_ref()))
    }
}
impl AsRef<[u8]> for Address {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

#[cfg(test)]
pub mod test {
    use super::*;
    use quickcheck::{Arbitrary, Gen};

    impl Arbitrary for Address {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            Address(Arbitrary::arbitrary(g))
        }
    }

}
