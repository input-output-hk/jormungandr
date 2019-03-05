use super::hex;
use std::fmt;
use std::marker::PhantomData;

use cryptoxide::util::fixed_time_eq;

pub const SIGNATURE_SIZE: usize = 64;

pub enum SignatureError {
    InvalidLength(usize),
}

/// a signature with an associated type tag
///
#[derive(Clone)]
pub struct Signature<T> {
    bytes: [u8; SIGNATURE_SIZE],
    _phantom: PhantomData<T>,
}
impl<T> Signature<T> {
    pub fn from_bytes(bytes: [u8; SIGNATURE_SIZE]) -> Self {
        Signature {
            bytes: bytes,
            _phantom: PhantomData,
        }
    }

    pub fn from_slice(bytes: &[u8]) -> Result<Self, SignatureError> {
        if bytes.len() != SIGNATURE_SIZE {
            return Err(SignatureError::InvalidLength(bytes.len()));
        }
        let mut buf = [0u8; SIGNATURE_SIZE];
        buf[..].clone_from_slice(bytes);
        Ok(Self::from_bytes(buf))
    }

    pub fn coerce<R>(self) -> Signature<R> {
        Signature::<R>::from_bytes(self.bytes)
    }

    pub fn to_bytes<'a>(&'a self) -> &'a [u8; SIGNATURE_SIZE] {
        &self.bytes
    }
}
impl<T> PartialEq for Signature<T> {
    fn eq(&self, rhs: &Signature<T>) -> bool {
        fixed_time_eq(self.as_ref(), rhs.as_ref())
    }
}
impl<T> Eq for Signature<T> {}
impl<T> fmt::Display for Signature<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", hex::encode(self.as_ref()))
    }
}
impl<T> fmt::Debug for Signature<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", hex::encode(self.as_ref()))
    }
}
impl<T> AsRef<[u8]> for Signature<T> {
    fn as_ref(&self) -> &[u8] {
        &self.bytes
    }
}
