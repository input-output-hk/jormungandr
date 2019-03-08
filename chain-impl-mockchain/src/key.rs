//! Module provides cryptographic utilities and types related to
//! the user keys.
//!
use chain_core::property;
use chain_crypto as crypto;
use chain_crypto::{
    AsymmetricKey, KeyEvolvingSignatureAlgorithm, SigningAlgorithm, VerificationAlgorithm,
};

use std::str::FromStr;

pub type SpendingPublicKey = crypto::PublicKey<crypto::Ed25519Extended>;
pub type SpendingSecretKey = crypto::SecretKey<crypto::Ed25519Extended>;
pub type SpendingSignature<T> = crypto::Signature<T, crypto::Ed25519Extended>;

#[inline]
pub fn serialize_public_key<A: AsymmetricKey, W: std::io::Write>(
    key: &crypto::PublicKey<A>,
    mut writer: W,
) -> Result<(), std::io::Error> {
    let size: usize = std::mem::size_of_val(key);
    assert!(size == 32);
    writer.write_all(key.as_ref())?;
    Ok(())
}
#[inline]
pub fn serialize_signature<A: VerificationAlgorithm, T, W: std::io::Write>(
    signature: &crypto::Signature<T, A>,
    mut writer: W,
) -> Result<(), std::io::Error> {
    let size: usize = std::mem::size_of_val(signature);
    assert!(size == 64);
    writer.write_all(signature.as_ref())?;
    Ok(())
}
#[inline]
pub fn deserialize_public_key<A, R>(mut reader: R) -> Result<crypto::PublicKey<A>, std::io::Error>
where
    A: AsymmetricKey,
    R: std::io::BufRead,
{
    let size: usize = std::mem::size_of::<crypto::PublicKey<A>>();
    assert!(size == 32);
    let mut buffer = vec![0; size];
    reader.read_exact(&mut buffer)?;
    crypto::PublicKey::from_bytes(&buffer)
        .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, Box::new(err)))
}
#[inline]
pub fn deserialize_signature<A, T, R>(
    mut reader: R,
) -> Result<crypto::Signature<T, A>, std::io::Error>
where
    A: VerificationAlgorithm,
    R: std::io::BufRead,
{
    let size: usize = std::mem::size_of::<crypto::Signature<T, A>>();
    assert!(size == 64);
    let mut buffer = vec![0; 64];
    reader.read_exact(&mut buffer)?;
    crypto::Signature::from_bytes(&buffer)
        .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, Box::new(err)))
}

pub fn make_signature<T, A>(
    spending_key: &crypto::SecretKey<A>,
    data: &T,
) -> crypto::Signature<T, A>
where
    A: SigningAlgorithm,
    T: property::Serialize,
{
    let bytes = data.serialize_as_vec().unwrap();
    crypto::Signature::generate(spending_key, &bytes).coerce()
}

pub fn make_signature_update<T, A>(
    spending_key: &mut crypto::SecretKey<A>,
    data: &T,
) -> crypto::Signature<T, A>
where
    A: KeyEvolvingSignatureAlgorithm,
    T: property::Serialize,
{
    let bytes = data.serialize_as_vec().unwrap();
    crypto::Signature::generate_update(spending_key, &bytes)
}

pub fn verify_signature<T, A>(
    signature: &crypto::Signature<T, A>,
    public_key: &crypto::PublicKey<A>,
    data: &T,
) -> crypto::Verification
where
    A: VerificationAlgorithm,
    T: property::Serialize,
{
    let bytes = data.serialize_as_vec().unwrap();
    signature.clone().coerce().verify(public_key, &bytes)
}

/// A serializable type T with a signature.
pub struct Signed<T, A: SigningAlgorithm> {
    pub data: T,
    pub sig: crypto::Signature<T, A>,
}

impl<T: property::Serialize, A: SigningAlgorithm> Signed<T, A> {
    pub fn new(secret_key: &crypto::SecretKey<A>, data: T) -> Self {
        let bytes = data.serialize_as_vec().unwrap();
        let signature = crypto::Signature::generate(secret_key, &bytes).coerce();
        Signed {
            data: data,
            sig: signature,
        }
    }
}

impl<T: property::Serialize, A: SigningAlgorithm> property::Serialize for Signed<T, A>
where
    std::io::Error: From<T::Error>,
{
    type Error = std::io::Error;
    fn serialize<W: std::io::Write>(&self, mut writer: W) -> Result<(), Self::Error> {
        self.data.serialize(&mut writer)?;
        serialize_signature(&self.sig, &mut writer)?;
        Ok(())
    }
}

impl<T: property::Deserialize, A: SigningAlgorithm> property::Deserialize for Signed<T, A>
where
    std::io::Error: From<T::Error>,
{
    type Error = std::io::Error;

    fn deserialize<R: std::io::BufRead>(mut reader: R) -> Result<Self, Self::Error> {
        Ok(Signed {
            data: T::deserialize(&mut reader)?,
            sig: deserialize_signature(&mut reader)?,
        })
    }
}

impl<T: PartialEq, A: SigningAlgorithm> PartialEq<Self> for Signed<T, A> {
    fn eq(&self, other: &Self) -> bool {
        self.data.eq(&other.data) && self.sig.as_ref() == other.sig.as_ref()
    }
}
impl<T: PartialEq, A: SigningAlgorithm> Eq for Signed<T, A> {}
impl<T: std::fmt::Debug, A: SigningAlgorithm> std::fmt::Debug for Signed<T, A> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "Signed ( data: {:?}, signature: {:?} )",
            self.data,
            self.sig.as_ref()
        )
    }
}
impl<T: Clone, A: SigningAlgorithm> Clone for Signed<T, A> {
    fn clone(&self) -> Self {
        Signed {
            data: self.data.clone(),
            sig: self.sig.clone(),
        }
    }
}

/// Hash that is used as an address of the various components.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Hash(crypto::Blake2b256);
impl Hash {
    pub fn hash_bytes(bytes: &[u8]) -> Self {
        Hash(crypto::Blake2b256::new(bytes))
    }
}

impl property::Serialize for Hash {
    type Error = std::io::Error;
    fn serialize<W: std::io::Write>(&self, mut writer: W) -> Result<(), Self::Error> {
        writer.write_all(self.0.as_hash_bytes())?;
        Ok(())
    }
}

impl property::Deserialize for Hash {
    type Error = std::io::Error;
    fn deserialize<R: std::io::BufRead>(mut reader: R) -> Result<Self, Self::Error> {
        let mut buffer = [0; crypto::Blake2b256::HASH_SIZE];
        reader.read_exact(&mut buffer)?;
        Ok(Hash(crypto::Blake2b256::from(buffer)))
    }
}

impl property::BlockId for Hash {
    fn zero() -> Hash {
        Hash(crypto::Blake2b256::from([0; crypto::Blake2b256::HASH_SIZE]))
    }
}

impl property::MessageId for Hash {}
impl property::TransactionId for Hash {}

impl AsRef<[u8]> for Hash {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl From<crypto::Blake2b256> for Hash {
    fn from(hash: crypto::Blake2b256) -> Self {
        Hash(hash)
    }
}

impl std::fmt::Display for Hash {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for Hash {
    type Err = crypto::hash::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Hash(crypto::Blake2b256::from_str(s)?))
    }
}

#[cfg(test)]
pub mod test {
    use super::*;
    use quickcheck::{Arbitrary, Gen};

    pub fn arbitrary_secret_key<A, G>(g: &mut G) -> crypto::SecretKey<A>
    where
        A: AsymmetricKey,
        G: Gen,
    {
        use rand_chacha::ChaChaRng;
        use rand_core::SeedableRng;
        let mut seed = [0; 32];
        for byte in seed.iter_mut() {
            *byte = Arbitrary::arbitrary(g);
        }
        let mut rng = ChaChaRng::from_seed(seed);
        crypto::SecretKey::generate(&mut rng)
    }

    impl Arbitrary for Hash {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            let bytes: Vec<u8> = Arbitrary::arbitrary(g);
            Hash::hash_bytes(&bytes)
        }
    }

    impl<A: SigningAlgorithm + 'static, T: property::Serialize + Arbitrary> Arbitrary for Signed<T, A>
    where
        A::Signature: Send,
    {
        fn arbitrary<G>(g: &mut G) -> Self
        where
            G: Gen,
        {
            let sk = arbitrary_secret_key(g);
            let data = T::arbitrary(g);
            Signed::new(&sk, data)
        }
    }
}
