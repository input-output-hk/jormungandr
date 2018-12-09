use curve25519_dalek::scalar::Scalar;
use curve25519_dalek::ristretto::RistrettoPoint;
use sha2::{Sha512, Digest};

type Point = RistrettoPoint;

/// Proof of discrete logarithm equivalence
#[derive(Clone)]
pub struct Proof {
    c: Challenge,
    z: Scalar,
}

const PROOF_SIZE : usize = 64; // Scalar is 32 bytes

impl Proof {
    pub fn to_bytes(&self, output: &mut [u8]) {
        assert_eq!(output.len(), PROOF_SIZE);
        output[0..32].copy_from_slice(self.c.0.as_bytes());
        output[32..64].copy_from_slice(self.z.as_bytes());
    }

    pub fn from_bytes(slice: &[u8]) -> Option<Self> {
        if slice.len() != PROOF_SIZE {
            return None;
        }
        let mut c_array = [0u8;32];
        c_array.copy_from_slice(&slice[0..32]);
        let c = Scalar::from_canonical_bytes(c_array)?;

        let mut z_array = [0u8;32];
        z_array.copy_from_slice(&slice[32..64]);
        let z = Scalar::from_canonical_bytes(z_array)?;

        let proof = Proof { c: Challenge(c), z: z };
        Some(proof)
    }
}

/// Parameters for DLEQ where g1^a = h1, h2^a = h2
pub struct DLEQ<'a> {
    pub g1: &'a RistrettoPoint,
    pub h1: &'a RistrettoPoint,
    pub g2: &'a RistrettoPoint,
    pub h2: &'a RistrettoPoint,
}

#[derive(Clone,PartialEq,Eq)]
struct Challenge(Scalar);

fn challenge(h1: &Point, h2: &Point, a1: &Point, a2: &Point) -> Challenge {
    let mut d = Sha512::new();
    d.input(h1.compress().as_bytes());
    d.input(h2.compress().as_bytes());
    d.input(a1.compress().as_bytes());
    d.input(a2.compress().as_bytes());
    Challenge(Scalar::from_hash(d))
}

/// Generate a zero knowledge of discrete log equivalence
///
/// where:
/// * g1^a = h1
/// * g2^a = h2
pub fn generate(w: &Scalar, a: &Scalar, dleq: &DLEQ) -> Proof {
    let a1 = dleq.g1 * w;
    let a2 = dleq.g2 * w;
    let c = challenge(&dleq.h1, &dleq.h2, &a1, &a2);
    let z = w + a * c.0;
    Proof { c, z }
}

/// Verify a zero knowledge proof of discrete log equivalence
pub fn verify(dleq: &DLEQ, proof: &Proof) -> bool {
    let r1 = dleq.g1 * proof.z;
    let r2 = dleq.g2 * proof.z;
    let a1 = r1 - (dleq.h1 * proof.c.0);
    let a2 = r2 - (dleq.h2 * proof.c.0);
    // no need for constant time equality because of the hash in challenge()
    challenge(&dleq.h1, &dleq.h2, &a1, &a2) == proof.c
}

#[cfg(test)]
mod tests {
    use curve25519_dalek::constants::RISTRETTO_BASEPOINT_POINT;
    use curve25519_dalek::{scalar::Scalar, ristretto::RistrettoPoint};
    use sha2::{Sha512};
    use rand::OsRng;

    use super::{DLEQ, generate, verify};

    #[test]
    #[allow(non_snake_case)]
    pub fn it_works() {
        let G = &RISTRETTO_BASEPOINT_POINT;
        let H = RistrettoPoint::hash_from_bytes::<Sha512>(G.compress().as_bytes());
        let mut csprng: OsRng = OsRng::new().unwrap();

        let a = Scalar::random(&mut csprng);
        let w = Scalar::random(&mut csprng);

        let dleq = DLEQ { g1: G, h1: &(G * a), g2: &H, h2: &(H * a) };
        let proof = generate(&w, &a, &dleq);
        assert_eq!(verify(&dleq, &proof), true);

        let dleq_bad = DLEQ { g1: G, h1: &(G * a), g2: &H, h2: &(H * w) };

        assert_eq!(verify(&dleq_bad, &proof), false);
    }
}
