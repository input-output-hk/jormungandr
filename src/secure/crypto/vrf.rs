//! Verifiable Random Function (VRF) implementation
//! using the 2-Hash-DH verifiable oblivious PRF
//! defined in the Ouroboros Praos paper

use rand::{CryptoRng, Rng};
use curve25519_dalek::scalar::Scalar;
use curve25519_dalek::ristretto::RistrettoPoint;
use curve25519_dalek::constants::RISTRETTO_BASEPOINT_POINT;
use sha2::Digest;
use sha2::Sha512;

use super::dleq;

type Point = RistrettoPoint;

/// VRF Secret Key
#[derive(Clone)]
pub struct SecretKey {
    secret: Scalar,
    public: Point,
}

/// VRF Public Key
#[derive(Clone,PartialEq,Eq)]
pub struct PublicKey(Point);

/// VRF Proof of generation
#[derive(Clone)]
pub struct Proof {
    u: Point,
    dleq: dleq::Proof,
}

/// VRF output for a given input
#[derive(Clone)]
pub struct Output([u8; 64]);

impl PartialEq for Output {
    fn eq(&self, rhs: &Output) -> bool {
        &self.0[..] == &rhs.0[..]
    }
}
impl Eq for Output {}
impl From<Sha512> for Output {
    fn from(f: Sha512) -> Self {
        let mut a = [0u8;64];
        let output = f.result();
        a.copy_from_slice(output.as_slice());
        Output(a)
    }
}

impl SecretKey {
    /// Create a new random secret key
    pub fn random<T: Rng+CryptoRng>(rng: &mut T) -> Self {
        let sk = Scalar::random(rng);
        let pk = RISTRETTO_BASEPOINT_POINT * sk;
        SecretKey {
            secret: sk,
            public: pk,
        }
    }

    pub fn to_bytes(&self) -> [u8;32] {
        let mut v = [0u8;32];
        v.copy_from_slice(self.secret.as_bytes());
        v
    }

    pub fn from_bytes(bytes: [u8;32]) -> Option<Self> {
        let sk = Scalar::from_canonical_bytes(bytes)?;
        let pk = RISTRETTO_BASEPOINT_POINT * sk;
        Some(SecretKey { secret: sk, public: pk })
    }

    /// initialize a slice to a specific number of bytes, and return the associated proof
    ///
    /// the proof is randomized, so need a freshly randomly scalar for random.
    pub fn evaluate(&self, r: &Scalar, data: &[u8]) -> (Output, Proof) {
        let m_point = make_message_hash_point(data);
        let u = m_point * self.secret;
        let y = make_output(data, &u);
        let dleq = dleq::DLEQ {
            g1: &RISTRETTO_BASEPOINT_POINT,
            h1: &self.public,
            g2: &m_point,
            h2: &u,
        };
        let dleq_proof = dleq::generate(&r, &self.secret, &dleq);
        let proof = Proof {
            u: u,
            dleq: dleq_proof,
        };

        (y, proof)
    }

    pub fn evaluate_simple<T: Rng+CryptoRng>(&self, rng: &mut T, data: &[u8]) -> (Output, Proof) {
        let w = Scalar::random(rng);
        self.evaluate(&w, data)
    }

    /// Get the public key associated with a secret key
    pub fn public(&self) -> PublicKey {
        PublicKey(self.public.clone())
    }
}

impl Proof {
    /// Verify a proof for a given public key and a data slice
    pub fn verify(&self, public_key: &PublicKey, data: &[u8], output: &Output) -> bool {
        let dleq = dleq::DLEQ {
            g1: &RISTRETTO_BASEPOINT_POINT,
            h1: &public_key.0,
            g2: &make_message_hash_point(data),
            h2: &self.u,
        };
        output == &make_output(data, &self.u) && dleq::verify(&dleq, &self.dleq)
    }
}

// hash data and u to create the output
fn make_output(data: &[u8], point: &Point) -> Output {
    let mut c = Sha512::new();
    c.input(data);
    c.input(point.compress().as_bytes());
    c.into()
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
    use rand::OsRng;
    use super::{SecretKey};

    #[test]
    fn it_works() {
        let mut csprng: OsRng = OsRng::new().unwrap();
        let sk = SecretKey::random(&mut csprng);
        let pk = sk.public();

        let sk_other = SecretKey::random(&mut csprng);
        let pk_other = sk_other.public();

        let mut b1 = [0u8; 10];
        for i in b1.iter_mut() { *i = rand::random() }
        let mut b2 = [0u8; 10];
        for i in b2.iter_mut() { *i = rand::random() }

        let (output, proof) = sk.evaluate_simple(&mut csprng, &b1[..]);

        // make sure the test pass
        assert_eq!(proof.verify(&pk, &b1[..], &output), true);

        // now try with false positive
        assert_eq!(proof.verify(&pk, &b2[..], &output), false);
        assert_eq!(proof.verify(&pk_other, &b1[..], &output), false);
        assert_eq!(proof.verify(&pk_other, &b2[..], &output), false);
    }
}
