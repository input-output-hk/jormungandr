//! Module provides cryptographic utilities and types related to
//! the user keys.
//!
use cardano::hash;
use cardano::redeem as crypto;
use chain_core::property;

/// Public key of the entity.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PublicKey(pub crypto::PublicKey);
impl PublicKey {
    pub fn verify(&self, message: &[u8], signature: &Signature) -> bool {
        self.0.verify(&signature.0, message)
    }

    pub fn from_bytes(bytes: [u8; crypto::PUBLICKEY_SIZE]) -> Self {
        PublicKey(crypto::PublicKey::from_bytes(bytes))
    }

    pub fn from_hex(string: &str) -> Result<Self, cardano::redeem::Error> {
        Ok(PublicKey(crypto::PublicKey::from_hex(string)?))
    }

    /// Convenience function to verify a serialize object.
    pub fn serialize_and_verify<T: property::Serialize>(
        &self,
        t: &T,
        signature: &Signature,
    ) -> bool {
        self.verify(&t.serialize_as_vec().unwrap(), signature)
    }
}

/// Private key of the entity.
#[derive(Debug, Clone)]
pub struct PrivateKey(crypto::PrivateKey);
impl PrivateKey {
    pub fn public(&self) -> PublicKey {
        PublicKey(self.0.public())
    }
    pub fn sign(&self, data: &[u8]) -> Signature {
        Signature(self.0.sign(data))
    }
    pub fn normalize_bytes(xprv: [u8; crypto::PRIVATEKEY_SIZE]) -> Self {
        PrivateKey(crypto::PrivateKey::normalize_bytes(xprv))
    }

    pub fn from_hex(input: &str) -> Result<Self, cardano::redeem::Error> {
        Ok(PrivateKey(crypto::PrivateKey::from_hex(&input)?))
    }

    /// Convenience function to sign a serialize object.
    pub fn serialize_and_sign<T: property::Serialize>(&self, t: &T) -> Signature {
        self.sign(&t.serialize_as_vec().unwrap())
    }
}

///
#[derive(Debug, Clone)]
pub struct KeyPair(PrivateKey, PublicKey);
impl KeyPair {
    pub fn private_key(&self) -> &PrivateKey {
        &self.0
    }
    pub fn public_key(&self) -> &PublicKey {
        &self.1
    }
    pub fn into_keys(self) -> (PrivateKey, PublicKey) {
        (self.0, self.1)
    }
}

/// Cryptographic signature.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signature(crypto::Signature);
impl Signature {
    pub fn from_bytes(bytes: [u8; crypto::SIGNATURE_SIZE]) -> Self {
        Signature(crypto::Signature::from_bytes(bytes))
    }
}

/// A serializable type T with a signature.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signed<T> {
    pub data: T,
    pub sig: Signature,
}

impl<T: property::Serialize> property::Serialize for Signed<T>
where
    std::io::Error: From<T::Error>,
{
    type Error = std::io::Error;
    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        use chain_core::packer::*;
        let mut codec = Codec::from(writer);
        self.data.serialize(&mut codec)?;
        self.sig.serialize(&mut codec)?;
        Ok(())
    }
}

impl<T: property::Deserialize> property::Deserialize for Signed<T>
where
    std::io::Error: From<T::Error>,
{
    type Error = std::io::Error;

    fn deserialize<R: std::io::BufRead>(reader: R) -> Result<Self, Self::Error> {
        use chain_core::packer::*;
        let mut codec = Codec::from(reader);
        Ok(Signed {
            data: T::deserialize(&mut codec)?,
            sig: Signature::deserialize(&mut codec)?,
        })
    }
}

/// Hash that is used as an address of the various components.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Hash(hash::Blake2b256);
impl Hash {
    pub fn hash_bytes(bytes: &[u8]) -> Self {
        Hash(hash::Blake2b256::new(bytes))
    }
}

impl property::Serialize for PublicKey {
    type Error = std::io::Error;
    fn serialize<W: std::io::Write>(&self, mut writer: W) -> Result<(), Self::Error> {
        writer.write_all(self.0.as_ref())?;
        Ok(())
    }
}
impl property::Serialize for Signature {
    type Error = std::io::Error;
    fn serialize<W: std::io::Write>(&self, mut writer: W) -> Result<(), Self::Error> {
        writer.write_all(self.0.as_ref())?;
        Ok(())
    }
}
impl property::Serialize for Hash {
    type Error = std::io::Error;
    fn serialize<W: std::io::Write>(&self, mut writer: W) -> Result<(), Self::Error> {
        writer.write_all(self.0.as_hash_bytes())?;
        Ok(())
    }
}

impl property::Deserialize for PublicKey {
    type Error = std::io::Error;
    fn deserialize<R: std::io::BufRead>(mut reader: R) -> Result<Self, Self::Error> {
        let mut buffer = [0; crypto::PUBLICKEY_SIZE];
        reader.read_exact(&mut buffer)?;
        Ok(crypto::PublicKey::from_bytes(buffer).into())
    }
}
impl property::Deserialize for Signature {
    type Error = std::io::Error;
    fn deserialize<R: std::io::BufRead>(mut reader: R) -> Result<Self, Self::Error> {
        let mut buffer = [0; crypto::SIGNATURE_SIZE];
        reader.read_exact(&mut buffer)?;
        Ok(crypto::Signature::from_bytes(buffer).into())
    }
}
impl property::Deserialize for Hash {
    type Error = std::io::Error;
    fn deserialize<R: std::io::BufRead>(mut reader: R) -> Result<Self, Self::Error> {
        let mut buffer = [0; hash::Blake2b256::HASH_SIZE];
        reader.read_exact(&mut buffer)?;
        Ok(Hash(hash::Blake2b256::from(buffer)))
    }
}

impl property::BlockId for Hash {}
impl property::TransactionId for Hash {}

impl AsRef<[u8]> for PrivateKey {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}
impl AsRef<[u8]> for PublicKey {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}
impl AsRef<[u8]> for Signature {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}
impl AsRef<[u8]> for Hash {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl From<crypto::PublicKey> for PublicKey {
    fn from(signature: crypto::PublicKey) -> Self {
        PublicKey(signature)
    }
}
impl From<crypto::PrivateKey> for PrivateKey {
    fn from(signature: crypto::PrivateKey) -> Self {
        PrivateKey(signature)
    }
}
impl From<crypto::Signature> for Signature {
    fn from(signature: crypto::Signature) -> Self {
        Signature(signature)
    }
}
impl From<hash::Blake2b256> for Hash {
    fn from(hash: hash::Blake2b256) -> Self {
        Hash(hash)
    }
}

impl std::fmt::Display for Hash {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use quickcheck::{Arbitrary, Gen, TestResult};

    impl Arbitrary for PublicKey {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            let mut xpub = [0; crypto::PUBLICKEY_SIZE];
            for byte in xpub.iter_mut() {
                *byte = Arbitrary::arbitrary(g);
            }
            PublicKey(crypto::PublicKey::from_bytes(xpub))
        }
    }

    impl Arbitrary for PrivateKey {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            let mut xprv = [0; crypto::PRIVATEKEY_SIZE];
            for byte in xprv.iter_mut() {
                *byte = Arbitrary::arbitrary(g);
            }
            PrivateKey(crypto::PrivateKey::normalize_bytes(xprv))
        }
    }

    impl Arbitrary for KeyPair {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            let mut prv = [0; crypto::PRIVATEKEY_SIZE];
            for byte in prv.iter_mut() {
                *byte = Arbitrary::arbitrary(g);
            }
            let sk = PrivateKey(crypto::PrivateKey::normalize_bytes(prv));
            let pk = sk.public();
            KeyPair(sk, pk)
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

    quickcheck! {
        fn keypair_signing_ok(input: (KeyPair, Vec<u8>)) -> bool {
            let (sk, pk) = input.0.into_keys();
            let data = input.1;

            let signature = sk.sign(&data);
            pk.verify(&data, &signature)
        }
        fn keypair_signing_ko(input: (PrivateKey, PublicKey, Vec<u8>)) -> bool {
            let (sk, pk) = (input.0, input.1);
            let data = input.2;

            let signature = sk.sign(&data);
            ! pk.verify(&data, &signature)
        }

        fn public_key_encode_decode(public_key: PublicKey) -> TestResult {
            property::testing::serialization_bijection(public_key)
        }
        fn signature_encode_decode(signature: Signature) -> TestResult {
            property::testing::serialization_bijection(signature)
        }
        fn hash_encode_decode(hash: Hash) -> TestResult {
            property::testing::serialization_bijection(hash)
        }
    }
}
