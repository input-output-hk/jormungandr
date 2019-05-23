//! Simple Assymmetric locking mechanism using:
//!
//! * ristretto-curve25519 for DH
//! * HKDF for KDF
//! * chacha20poly1305 for symmetric encryption algorithm
//!
use cryptoxide::chacha20poly1305::ChaCha20Poly1305;
use cryptoxide::hkdf::hkdf_expand;
use cryptoxide::sha2;
use curve25519_dalek::constants::RISTRETTO_BASEPOINT_POINT;
use curve25519_dalek::ristretto::CompressedRistretto;
use curve25519_dalek::ristretto::RistrettoPoint;
use curve25519_dalek::scalar::Scalar;
use rand_core::{CryptoRng, RngCore};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecryptionError {
    DataTooSmall,
    PointInvalid,
    TagMismatch,
}

fn shared_key_to_symmetric_key(app_level_info: &[u8], p: &RistrettoPoint) -> ChaCha20Poly1305 {
    // use the compressed point as PRK directly
    let prk = p.compress().to_bytes();
    let mut symkey = [0u8; 32 + 12];
    hkdf_expand(sha2::Sha256::new(), &prk, app_level_info, &mut symkey);
    let ctx = ChaCha20Poly1305::new(&symkey[0..32], &symkey[32..], &[]);
    ctx
}

const SCHEME_OVERHEAD: usize = 48; // 32 bytes of public key + 16 bytes of tag

/// Encrypt data in an assymetric lock
///
/// # Return
///
/// the data encrypted with a ephemeral public key in prefix and
/// the poly1305 tag in suffix.
pub fn encrypt<R: RngCore + CryptoRng>(
    rng: &mut R,
    app_info: &[u8],
    receiver_pk: &RistrettoPoint,
    data: &[u8],
) -> Vec<u8> {
    // create a new ephemeral key and throw away the secret key keeping only the public key
    // and the shared key
    let r = Scalar::random(rng);
    let pk = RISTRETTO_BASEPOINT_POINT * r;
    let shared = r * receiver_pk;

    // Create a ChaCha20Poly1305 encryption context
    let mut context = shared_key_to_symmetric_key(app_info, &shared);

    // encrypt the data with the context
    let mut out = vec![0u8; data.len() + SCHEME_OVERHEAD];
    out[0..32].copy_from_slice(pk.compress().as_bytes());
    let (pk_and_encrypted, tag) = out.split_at_mut(32 + data.len());
    context.encrypt(data, &mut pk_and_encrypted[32..], tag);
    out
}

/// Decrypt data in the asymmetric lock. this is the dual of 'encrypt'.
/// The data should in the form:
///
/// ```text
///     EPHEMERAL_PUBLIC_KEY || ENCRYPTED-DATA || POLY1305-TAG
/// ```
///
/// # Return
///
/// Error if:
/// * data is too small
/// * point is not in the first format
/// * tag don't match
/// Success otherwise
///
/// # Panics
///
/// If output 'out' is not 48 bytes less than 'data'
///
pub fn decrypt(
    app_info: &[u8],
    sk: &Scalar,
    data: &[u8],
    out: &mut [u8],
) -> Result<(), DecryptionError> {
    if data.len() < SCHEME_OVERHEAD {
        return Err(DecryptionError::DataTooSmall);
    }
    assert_eq!(data.len() - SCHEME_OVERHEAD, out.len());

    let pk_data = &data[0..32];
    let payload = &data[32..data.len() - 16];
    let tag = &data[data.len() - 16..];

    let pk = CompressedRistretto::from_slice(pk_data);
    let shared = sk * pk.decompress().ok_or(DecryptionError::PointInvalid)?;

    let mut context = shared_key_to_symmetric_key(app_info, &shared);
    if !context.decrypt(payload, out, tag) {
        return Err(DecryptionError::TagMismatch);
    }
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    pub fn it_works() {
        let mut r = rand::thread_rng();

        // create a random keypair
        let sk_receiver = Scalar::random(&mut r);
        let pk_receiver = RISTRETTO_BASEPOINT_POINT * sk_receiver;

        let app_info = b"hello";
        let msg = b"message";
        let mut out = vec![0; msg.len()];
        let encrypted = encrypt(&mut r, app_info, &pk_receiver, msg);
        decrypt(app_info, &sk_receiver, &encrypted, &mut out).unwrap();
        assert_eq!(out, msg);
    }
}
