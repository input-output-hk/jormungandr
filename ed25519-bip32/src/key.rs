use std::fmt;

use cryptoxide::ed25519;
use cryptoxide::ed25519::signature_extended;
use cryptoxide::util::fixed_time_eq;

use std::hash::{Hash, Hasher};

use super::derivation::{self, DerivationError, DerivationIndex, DerivationScheme};
use super::hex;
use super::securemem;
use super::signature::Signature;

pub const XPRV_SIZE: usize = 96;
pub const XPUB_SIZE: usize = 64;
pub const PUBLIC_KEY_SIZE: usize = 32;
pub const CHAIN_CODE_SIZE: usize = 32;

#[derive(Debug, PartialEq, Eq)]
pub enum PrivateKeyError {
    LengthInvalid(usize),
    HighestBitsInvalid,
    LowestBitsInvalid,
}

pub enum PublicKeyError {
    LengthInvalid(usize),
}

/// HDWallet extended private key
///
/// Effectively this is ed25519 extended secret key (64 bytes) followed by a chain code (32 bytes)
pub struct XPrv([u8; XPRV_SIZE]);
impl XPrv {
    /// takes the given raw bytes and perform some modifications to normalize
    /// to a valid XPrv.
    ///
    pub fn normalize_bytes(mut bytes: [u8; XPRV_SIZE]) -> Self {
        bytes[0] &= 0b1111_1000;
        bytes[31] &= 0b0001_1111;
        bytes[31] |= 0b0100_0000;;

        Self::from_bytes(bytes)
    }

    // Create a XPrv from the given bytes.
    //
    // This function does not perform any validity check and should not be used outside
    // of this crate.
    pub(crate) fn from_bytes(bytes: [u8; XPRV_SIZE]) -> Self {
        XPrv(bytes)
    }

    /// Create a `XPrv` by taking ownership of the given array
    ///
    /// This function may returns an error if it does not have the expected
    /// format.
    pub fn from_bytes_verified(bytes: [u8; XPRV_SIZE]) -> Result<Self, PrivateKeyError> {
        let scalar = &bytes[0..32];
        let last = scalar[31];
        let first = scalar[0];

        if (last & 0b1110_0000) != 0b0100_0000 {
            return Err(PrivateKeyError::HighestBitsInvalid);
        }
        if (first & 0b0000_0111) != 0b0000_0000 {
            return Err(PrivateKeyError::LowestBitsInvalid);
        }

        Ok(XPrv(bytes))
    }

    pub fn from_slice_verified(bytes: &[u8]) -> Result<Self, PrivateKeyError> {
        if bytes.len() != XPRV_SIZE {
            return Err(PrivateKeyError::LengthInvalid(bytes.len()));
        }

        let mut buf = [0u8; XPRV_SIZE];
        buf[..].clone_from_slice(bytes);
        XPrv::from_bytes_verified(buf)
    }

    /// Create a `XPrv` from the given slice. This slice must be of size `XPRV_SIZE`
    /// otherwise it will return `Err`.
    ///
    fn from_slice(bytes: &[u8]) -> Result<Self, PrivateKeyError> {
        if bytes.len() != XPRV_SIZE {
            return Err(PrivateKeyError::LengthInvalid(bytes.len()));
        }
        let mut buf = [0u8; XPRV_SIZE];
        buf[..].clone_from_slice(bytes);
        Ok(XPrv::from_bytes(buf))
    }

    /// Get the associated `XPub`
    ///
    pub fn public(&self) -> XPub {
        let pk = mk_public_key(&self.as_ref()[0..64]);
        let mut out = [0u8; XPUB_SIZE];
        out[0..32].clone_from_slice(&pk);
        out[32..64].clone_from_slice(&self.as_ref()[64..]);
        XPub::from_bytes(out)
    }

    /// sign the given message with the `XPrv`.
    ///
    pub fn sign<T>(&self, message: &[u8]) -> Signature<T> {
        Signature::from_bytes(signature_extended(message, &self.as_ref()[0..64]))
    }

    /// verify a given signature
    ///
    pub fn verify<T>(&self, message: &[u8], signature: &Signature<T>) -> bool {
        let xpub = self.public();
        xpub.verify(message, signature)
    }

    pub fn derive(&self, scheme: DerivationScheme, index: DerivationIndex) -> Self {
        derivation::private(self, index, scheme)
    }

    pub fn get_extended(&self, out: &mut [u8; 64]) {
        out.clone_from_slice(&self.as_ref()[0..64])
    }
}
impl PartialEq for XPrv {
    fn eq(&self, rhs: &XPrv) -> bool {
        fixed_time_eq(self.as_ref(), rhs.as_ref())
    }
}
impl Eq for XPrv {}
impl Clone for XPrv {
    fn clone(&self) -> Self {
        Self::from_slice(self.as_ref()).expect("it is already a safely constructed XPrv")
    }
}
impl fmt::Debug for XPrv {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", hex::encode(self.as_ref()))
    }
}
impl fmt::Display for XPrv {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", hex::encode(self.as_ref()))
    }
}
impl AsRef<[u8]> for XPrv {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}
impl Drop for XPrv {
    fn drop(&mut self) {
        securemem::zero(&mut self.0);
    }
}

/// Extended Public Key (Point + ChainCode)
#[derive(Clone, Copy)]
pub struct XPub([u8; XPUB_SIZE]);
impl XPub {
    /// create a `XPub` by taking ownership of the given array
    pub fn from_bytes(bytes: [u8; XPUB_SIZE]) -> Self {
        XPub(bytes)
    }

    /// create a `XPub` from the given slice. This slice must be of size `XPUB_SIZE`
    /// otherwise it will return `Option::None`.
    ///
    pub fn from_slice(bytes: &[u8]) -> Result<Self, PublicKeyError> {
        if bytes.len() != XPUB_SIZE {
            return Err(PublicKeyError::LengthInvalid(bytes.len()));
        }
        let mut buf = [0u8; XPUB_SIZE];
        buf[..].clone_from_slice(bytes);
        Ok(Self::from_bytes(buf))
    }

    /// verify a signature
    ///
    pub fn verify<T>(&self, message: &[u8], signature: &Signature<T>) -> bool {
        ed25519::verify(message, &self.as_ref()[0..32], signature.as_ref())
    }

    pub fn derive(
        &self,
        scheme: DerivationScheme,
        index: DerivationIndex,
    ) -> Result<Self, DerivationError> {
        derivation::public(self, index, scheme)
    }

    pub fn get_without_chaincode(&self, out: &mut [u8; 32]) {
        out.clone_from_slice(&self.0[0..32])
    }
}
impl PartialEq for XPub {
    fn eq(&self, rhs: &XPub) -> bool {
        fixed_time_eq(self.as_ref(), rhs.as_ref())
    }
}
impl Eq for XPub {}
impl Hash for XPub {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write(&self.0)
    }
}
impl fmt::Display for XPub {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", hex::encode(self.as_ref()))
    }
}
impl fmt::Debug for XPub {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", hex::encode(self.as_ref()))
    }
}
impl AsRef<[u8]> for XPub {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

pub(crate) fn mk_xprv(out: &mut [u8; XPRV_SIZE], kl: &[u8], kr: &[u8], cc: &[u8]) {
    assert!(kl.len() == 32);
    assert!(kr.len() == 32);
    assert!(cc.len() == CHAIN_CODE_SIZE);

    out[0..32].clone_from_slice(kl);
    out[32..64].clone_from_slice(kr);
    out[64..96].clone_from_slice(cc);
}

pub(crate) fn mk_xpub(out: &mut [u8; XPUB_SIZE], pk: &[u8], cc: &[u8]) {
    assert!(pk.len() == 32);
    assert!(cc.len() == CHAIN_CODE_SIZE);

    out[0..32].clone_from_slice(pk);
    out[32..64].clone_from_slice(cc);
}

pub fn mk_public_key(extended_secret: &[u8]) -> [u8; PUBLIC_KEY_SIZE] {
    assert!(extended_secret.len() == 64);
    ed25519::to_public(extended_secret)
}
