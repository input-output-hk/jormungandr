use super::sign::{Ed25519, SignatureAlgorithm};

// General Key Evolving Signature algorithm Trait
pub trait KES {
    type Signature;
    type SecretKey;
    type PublicKey;

    /// Generate a signature for a given message and update the secret key to the next one
    fn sign_and_update(&self, secretkey: &mut Self::SecretKey, data: &[u8]) -> Self::Signature;

    /// Verify that a signature has been made by a given (public key, message)
    ///
    /// This is idential to usual asymetric cryptographic signature algorithm
    fn verify(&self, publickey: &Self::PublicKey, data: &[u8], signature: &Self::Signature)
        -> bool;
}

pub type MockKES = Ed25519;

impl KES for MockKES {
    type Signature = <Ed25519 as SignatureAlgorithm>::Signature;
    type SecretKey = <Ed25519 as SignatureAlgorithm>::SecretKey;
    type PublicKey = <Ed25519 as SignatureAlgorithm>::PublicKey;

    fn sign_and_update(&self, secretkey: &mut Self::SecretKey, data: &[u8]) -> Self::Signature {
        self.sign(secretkey, data)
    }

    fn verify(
        &self,
        publickey: &Self::PublicKey,
        data: &[u8],
        signature: &Self::Signature,
    ) -> bool {
        <Ed25519 as SignatureAlgorithm>::verify(self, publickey, data, signature)
    }
}
