//! Verifiable Random Function (VRF) implementation
//! using the 2-Hash-DH verifiable oblivious PRF
//! defined in the Ouroboros Praos paper

use curve25519_dalek::constants::RISTRETTO_BASEPOINT_POINT;
use curve25519_dalek::ristretto::CompressedRistretto;
use curve25519_dalek::ristretto::RistrettoPoint;
pub use curve25519_dalek::scalar::Scalar;
use generic_array::GenericArray;
use rand::{CryptoRng, Rng};
use sha2::Digest;
use sha2::Sha512;
use std::hash::{Hash, Hasher};

use super::dleq;
use crate::key::PublicKeyError;

type Point = RistrettoPoint;

/// VRF Secret Key
#[derive(Clone)]
pub struct SecretKey {
    secret: Scalar,
    public: Point,
}

impl AsRef<[u8]> for SecretKey {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

/// VRF Public Key
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicKey(Point, CompressedRistretto);

impl Hash for PublicKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_bytes().hash(state)
    }
}
impl AsRef<[u8]> for PublicKey {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

/// VRF Output (Point)
///
/// This is used to create an output generator tweaked by the VRF.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputSeed(Point);

/// VRF Proof of generation
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProvenOutputSeed {
    u: OutputSeed,
    dleq_proof: dleq::Proof,
}

pub const PROOF_SIZE: usize = 96;
pub const SECRET_SIZE: usize = 32;
pub const PUBLIC_SIZE: usize = 32;

impl SecretKey {
    /// Create a new random secret key
    pub fn random<T: Rng + CryptoRng>(mut rng: T) -> Self {
        let sk = Scalar::random(&mut rng);
        let pk = RISTRETTO_BASEPOINT_POINT * sk;
        SecretKey {
            secret: sk,
            public: pk,
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.secret.as_bytes()
    }

    /// Serialize the secret key in binary form
    pub fn to_bytes(&self) -> [u8; SECRET_SIZE] {
        let mut v = [0u8; SECRET_SIZE];
        v.copy_from_slice(self.secret.as_bytes());
        v
    }

    pub fn from_bytes(bytes: [u8; SECRET_SIZE]) -> Option<Self> {
        let sk = Scalar::from_canonical_bytes(bytes)?;
        let pk = RISTRETTO_BASEPOINT_POINT * sk;
        Some(SecretKey {
            secret: sk,
            public: pk,
        })
    }

    /// Get the verifiable output and the associated input base point.
    ///
    /// The following property hold between the return values:
    ///     Point * secret = OutputSeed
    pub fn verifiable_output(&self, input: &[u8]) -> (Point, OutputSeed) {
        let m_point = make_message_hash_point(input);
        let u = m_point * self.secret;
        (m_point, OutputSeed(u))
    }

    /// Create a proof, for the given parameters; no check is made to make sure it's correct
    ///
    /// the proof is randomized, so need a freshly randomly scalar for random.
    /// use 'proove_simple' to use a RNG and avoid generating this random.
    ///
    /// use 'evaluate' or 'evaluate_simple' for creating the proof directly from input
    pub fn proove(&self, r: &Scalar, m_point: Point, output: OutputSeed) -> ProvenOutputSeed {
        let dleq = dleq::DLEQ {
            g1: &RISTRETTO_BASEPOINT_POINT,
            h1: &self.public,
            g2: &m_point,
            h2: &output.0,
        };
        let dleq_proof = dleq::generate(&r, &self.secret, &dleq);
        let proof = ProvenOutputSeed {
            u: output.clone(),
            dleq_proof: dleq_proof,
        };

        proof
    }

    pub fn proove_simple<T: Rng + CryptoRng>(
        &self,
        rng: &mut T,
        m_point: Point,
        output: OutputSeed,
    ) -> ProvenOutputSeed {
        let w = Scalar::random(rng);
        self.proove(&w, m_point, output)
    }

    /// Generate a Proof
    ///
    /// the proof is randomized, so need a freshly randomly scalar for random.
    /// use 'evaluate_simple' for normal use.
    pub fn evaluate(&self, r: &Scalar, input: &[u8]) -> ProvenOutputSeed {
        let (m_point, output) = self.verifiable_output(input);
        self.proove(r, m_point, output)
    }

    pub fn evaluate_simple<T: Rng + CryptoRng>(
        &self,
        rng: &mut T,
        input: &[u8],
    ) -> ProvenOutputSeed {
        let (m_point, output) = self.verifiable_output(input);
        self.proove_simple(rng, m_point, output)
    }

    /// Get the public key associated with a secret key
    pub fn public(&self) -> PublicKey {
        PublicKey(self.public.clone(), self.public.compress())
    }
}

impl PublicKey {
    pub fn from_bytes(input: &[u8]) -> Result<Self, PublicKeyError> {
        if input.len() != PUBLIC_SIZE {
            return Err(PublicKeyError::SizeInvalid);
        }
        let ristretto = CompressedRistretto::from_slice(&input);
        match ristretto.decompress() {
            None => Err(PublicKeyError::StructureInvalid),
            Some(pk) => Ok(PublicKey(pk, ristretto)),
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.1.as_bytes()
    }

    pub fn to_buffer(&self, output: &mut [u8]) {
        assert_eq!(output.len(), PUBLIC_SIZE);
        output.copy_from_slice(self.0.compress().as_bytes())
    }
}

impl ProvenOutputSeed {
    /// Verify a proof for a given public key and a data slice
    pub fn verify(&self, public_key: &PublicKey, input: &[u8]) -> bool {
        let dleq = dleq::DLEQ {
            g1: &RISTRETTO_BASEPOINT_POINT,
            h1: &public_key.0,
            g2: &make_message_hash_point(input),
            h2: &self.u.0,
        };
        dleq::verify(&dleq, &self.dleq_proof)
    }

    pub fn to_bytes(&self, output: &mut [u8]) {
        assert_eq!(output.len(), PROOF_SIZE);
        output[0..32].copy_from_slice(self.u.0.compress().as_bytes());
        self.dleq_proof.to_bytes(&mut output[32..96]);
    }

    pub fn from_bytes_unverified(bytes: &[u8]) -> Option<Self> {
        if bytes.len() != PROOF_SIZE {
            return None;
        }
        let u = CompressedRistretto::from_slice(&bytes[0..32]).decompress()?;
        let proof = dleq::Proof::from_bytes(&bytes[32..])?;
        Some(ProvenOutputSeed {
            u: OutputSeed(u),
            dleq_proof: proof,
        })
    }

    pub fn from_bytes(public_key: &PublicKey, bytes: &[u8], input: &[u8]) -> Option<Self> {
        let pos = Self::from_bytes_unverified(bytes)?;
        if pos.verify(public_key, input) {
            Some(pos)
        } else {
            None
        }
    }

    pub fn to_verifiable_output(&self, public_key: &PublicKey, input: &[u8]) -> Option<OutputSeed> {
        if self.verify(public_key, input) {
            Some(self.u.clone())
        } else {
            None
        }
    }
}

impl OutputSeed {
    /// Create a new output generator using a simple digest seeding mechanism
    ///
    /// The digest returned can be used to generate multiple output given
    /// different suffix
    pub fn to_output_digest_generator<D: Digest>(&self, input: &[u8]) -> D {
        let mut c = <D as Digest>::new();
        c.input(input);
        c.input(self.0.compress().as_bytes());
        c
    }

    /// Get the output for this input and a known suffix
    pub fn to_output<D: Digest>(
        &self,
        input: &[u8],
        suffix: &[u8],
    ) -> GenericArray<u8, D::OutputSize> {
        let mut c = self.to_output_digest_generator::<D>(input);
        c.input(suffix);
        c.result()
    }
}

fn make_message_hash_point(data: &[u8]) -> Point {
    let m_hash = {
        let mut c = Sha512::new();
        c.input(data);
        c
    };
    Point::from_hash(m_hash)
}

#[cfg(test)]
mod tests {
    use super::SecretKey;
    use rand::rngs::OsRng;

    #[test]
    fn it_works() {
        let mut csprng: OsRng = OsRng::new().unwrap();
        let sk = SecretKey::random(&mut csprng);
        let pk = sk.public();

        let sk_other = SecretKey::random(&mut csprng);
        let pk_other = sk_other.public();

        let mut b1 = [0u8; 10];
        for i in b1.iter_mut() {
            *i = rand::random()
        }
        let mut b2 = [0u8; 10];
        for i in b2.iter_mut() {
            *i = rand::random()
        }

        let proof = sk.evaluate_simple(&mut csprng, &b1[..]);

        // make sure the test pass
        assert_eq!(proof.verify(&pk, &b1[..]), true);

        // now try with false positive
        assert_eq!(proof.verify(&pk, &b2[..]), false);
        assert_eq!(proof.verify(&pk_other, &b1[..]), false);
        assert_eq!(proof.verify(&pk_other, &b2[..]), false);
    }
}

#[cfg(test)]
#[cfg(feature = "with-bench")]
mod bench {
    use super::{PublicKey, SecretKey};
    use rand::OsRng;
    use test::Bencher;

    fn common() -> (OsRng, SecretKey, PublicKey, [u8; 10], [u8; 10]) {
        let mut csprng: OsRng = OsRng::new().unwrap();
        let sk = SecretKey::random(&mut csprng);
        let pk = sk.public();

        let sk_other = SecretKey::random(&mut csprng);
        let pk_other = sk_other.public();

        let mut b1 = [0u8; 10];
        for i in b1.iter_mut() {
            *i = rand::random()
        }
        let mut b2 = [0u8; 10];
        for i in b2.iter_mut() {
            *i = rand::random()
        }

        (csprng, sk, pk, b1, b2)
    }

    #[bench]
    fn generate(b: &mut test::Bencher) {
        let (mut csprng, sk, pk, b1, _) = common();

        b.iter(|| {
            let _ = sk.evaluate_simple(&mut csprng, &b1[..]);
        })
    }

    #[bench]
    fn verify_success(b: &mut test::Bencher) {
        let (mut csprng, sk, pk, b1, _) = common();
        let po = sk.evaluate_simple(&mut csprng, &b1[..]);

        b.iter(|| {
            let _ = po.verify(&pk, &b1[..]);
        })
    }

    #[bench]
    fn verify_fail(b: &mut test::Bencher) {
        let (mut csprng, sk, pk, b1, b2) = common();
        let (_, _, pk2, _, _) = common();
        let po = sk.evaluate_simple(&mut csprng, &b1[..]);

        b.iter(|| {
            let _ = po.verify(&pk2, &b1[..]);
        })
    }
}
