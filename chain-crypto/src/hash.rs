//! module to provide some handy interfaces atop the hashes so we have
//! the common interfaces for the project to work with.

use std::{
    fmt,
    hash::{Hash, Hasher},
    io::{BufRead, Write},
    result,
    str::FromStr,
};

use cryptoxide::blake2b::Blake2b;
use cryptoxide::digest::Digest;
use cryptoxide::sha3::Sha3;

use crate::hex;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum Error {
    InvalidHashSize(usize, usize),
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &Error::InvalidHashSize(sz, expected) => write!(
                f,
                "invalid hash size, expected {} but received {} bytes.",
                expected, sz
            ),
        }
    }
}
impl ::std::error::Error for Error {}

pub type Result<T> = result::Result<T, Error>;

/// defines a blake2b object
macro_rules! define_blake2b_new {
    ($hash_ty:ty) => {
        impl $hash_ty {
            pub fn new(buf: &[u8]) -> Self {
                let mut b2b = Blake2b::new(Self::HASH_SIZE);
                let mut out = [0; Self::HASH_SIZE];
                b2b.input(buf);
                b2b.result(&mut out);
                Self::from(out)
            }
        }
    };
}
macro_rules! define_hash_object {
    ($hash_ty:ty, $constructor:expr, $hash_size:ident) => {
        impl $hash_ty {
            pub const HASH_SIZE: usize = $hash_size;

            pub fn as_hash_bytes(&self) -> &[u8; Self::HASH_SIZE] {
                &self.0
            }

            pub fn try_from_slice(slice: &[u8]) -> Result<Self> {
                if slice.len() != Self::HASH_SIZE {
                    return Err(Error::InvalidHashSize(slice.len(), Self::HASH_SIZE));
                }
                let mut buf = [0; Self::HASH_SIZE];

                buf[0..Self::HASH_SIZE].clone_from_slice(slice);
                Ok(Self::from(buf))
            }
        }
        impl AsRef<[u8]> for $hash_ty {
            fn as_ref(&self) -> &[u8] {
                self.0.as_ref()
            }
        }
        impl From<$hash_ty> for [u8; $hash_size] {
            fn from(bytes: $hash_ty) -> Self {
                bytes.0
            }
        }
        impl From<[u8; Self::HASH_SIZE]> for $hash_ty {
            fn from(bytes: [u8; Self::HASH_SIZE]) -> Self {
                $constructor(bytes)
            }
        }
        impl Hash for $hash_ty {
            fn hash<H: Hasher>(&self, state: &mut H) {
                self.0.hash(state)
            }
        }
        impl fmt::Display for $hash_ty {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "{}", hex::encode(self.as_ref()))
            }
        }
        impl fmt::Debug for $hash_ty {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str(concat!(stringify!($hash_ty), "(0x"))?;
                write!(f, "{}", hex::encode(self.as_ref()))?;
                f.write_str(")")
            }
        }
    };
}

pub const HASH_SIZE_224: usize = 28;

pub const HASH_SIZE_256: usize = 32;

#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
pub struct Blake2b224([u8; HASH_SIZE_224]);
define_hash_object!(Blake2b224, Blake2b224, HASH_SIZE_224);
define_blake2b_new!(Blake2b224);

/// Blake2b 256 bits
#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
pub struct Blake2b256([u8; HASH_SIZE_256]);
define_hash_object!(Blake2b256, Blake2b256, HASH_SIZE_256);
define_blake2b_new!(Blake2b256);

#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
pub struct Sha3_256([u8; HASH_SIZE_256]);
define_hash_object!(Sha3_256, Sha3_256, HASH_SIZE_256);
impl Sha3_256 {
    pub fn new(buf: &[u8]) -> Self {
        let mut sh3 = Sha3::sha3_256();
        let mut out = [0; Self::HASH_SIZE];
        sh3.input(buf.as_ref());
        sh3.result(&mut out);
        Self::from(out)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn debug_blake2b_224() {
        let h = Blake2b224::new([0; 28].as_ref());
        assert_eq!(
            format!("{:?}", h),
            "Blake2b224(0x317512db8239e1f9c2549b04e8071f965983c938d3e649cec78532c7)",
        );
    }
}
