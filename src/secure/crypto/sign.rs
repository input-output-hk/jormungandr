use cryptoxide::ed25519;

// Asymmetrical Signature Algorithm Trait
pub trait SignatureAlgorithm {
    type Signature;
    type SecretKey;
    type PublicKey;

    /// Generate a signature for a given message and update the secret key to the next one
    fn sign(&self, secretkey: &Self::SecretKey, data: &[u8]) -> Self::Signature;

    /// Verify that a signature has been made by a given (public key, message)
    ///
    /// This is idential to usual asymetric cryptographic signature algorithm
    fn verify(&self, publickey: &Self::PublicKey, data: &[u8], signature: &Self::Signature) -> bool;
}

pub struct Ed25519;

impl SignatureAlgorithm for Ed25519 {
    type Signature = [u8;ed25519::SIGNATURE_LENGTH];
    type SecretKey = [u8;ed25519::PRIVATE_KEY_LENGTH];
    type PublicKey = [u8;ed25519::PUBLIC_KEY_LENGTH];

    fn sign(&self, secretkey: &Self::SecretKey, data: &[u8]) -> Self::Signature {
        ed25519::signature(data, secretkey)
    }

    fn verify(&self, publickey: &Self::PublicKey, data: &[u8], signature: &Self::Signature) -> bool {
        ed25519::verify(data, publickey, signature)
    }
}
