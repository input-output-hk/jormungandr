//! module to provide some handy interfaces atop the hashes so we have
//! the common interfaces for the project to work with.

use std::convert::TryFrom;
use std::hash::{Hash, Hasher};
use std::str::FromStr;
use std::{error, fmt, result};

use cryptoxide::blake2b::Blake2b;
use cryptoxide::digest::Digest as _;
use cryptoxide::sha3::Sha3;

use crate::bech32::{self, Bech32};
use crate::hash::{Blake2b256, Sha3_256};
use crate::hex;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Error {
    InvalidDigestSize { got: usize, expected: usize },
    InvalidHexEncoding(hex::DecodeError),
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::InvalidDigestSize { got: sz, expected } => write!(
                f,
                "invalid digest size, expected {} but received {} bytes.",
                expected, sz
            ),
            Error::InvalidHexEncoding(_) => write!(f, "invalid hex encoding for digest value"),
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Error::InvalidDigestSize {
                got: _,
                expected: _,
            } => None,
            Error::InvalidHexEncoding(err) => Some(err),
        }
    }
}

impl From<hex::DecodeError> for Error {
    fn from(err: hex::DecodeError) -> Self {
        Error::InvalidHexEncoding(err)
    }
}

#[derive(Debug, Copy, Clone)]
pub struct TryFromSliceError(());

impl fmt::Display for TryFromSliceError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt("could not convert slice to digest", f)
    }
}

pub trait DigestAlg {
    const HASH_SIZE: usize;
    type DigestData: Clone + PartialEq + Hash + AsRef<[u8]>;
    type DigestContext: Clone;

    fn try_from_slice(slice: &[u8]) -> Result<Self::DigestData, Error>;
    fn new() -> Self::DigestContext;
    fn append_data(ctx: &mut Self::DigestContext, data: &[u8]);
    fn finalize(ctx: Self::DigestContext) -> Self::DigestData;
}

impl DigestAlg for Blake2b256 {
    const HASH_SIZE: usize = 32;
    type DigestData = [u8; Self::HASH_SIZE];
    type DigestContext = Blake2b;

    fn try_from_slice(slice: &[u8]) -> Result<Self::DigestData, Error> {
        if slice.len() == Self::HASH_SIZE {
            let mut out = [0u8; Self::HASH_SIZE];
            out.copy_from_slice(slice);
            Ok(out)
        } else {
            Err(Error::InvalidDigestSize {
                expected: Self::HASH_SIZE,
                got: slice.len(),
            })
        }
    }

    fn new() -> Self::DigestContext {
        Blake2b::new(Self::HASH_SIZE)
    }

    fn append_data(ctx: &mut Self::DigestContext, data: &[u8]) {
        ctx.input(data)
    }

    fn finalize(mut ctx: Self::DigestContext) -> Self::DigestData {
        let mut out: Self::DigestData = [0; Self::HASH_SIZE];
        ctx.result(&mut out);
        out
    }
}

impl DigestAlg for Sha3_256 {
    const HASH_SIZE: usize = 32;
    type DigestData = [u8; Self::HASH_SIZE];
    type DigestContext = Sha3;

    fn try_from_slice(slice: &[u8]) -> Result<Self::DigestData, Error> {
        if slice.len() == Self::HASH_SIZE {
            let mut out = [0u8; Self::HASH_SIZE];
            out.copy_from_slice(slice);
            Ok(out)
        } else {
            Err(Error::InvalidDigestSize {
                expected: Self::HASH_SIZE,
                got: slice.len(),
            })
        }
    }

    fn new() -> Self::DigestContext {
        Sha3::sha3_256()
    }

    fn append_data(ctx: &mut Self::DigestContext, data: &[u8]) {
        ctx.input(data)
    }

    fn finalize(mut ctx: Self::DigestContext) -> Self::DigestData {
        let mut out: Self::DigestData = [0; Self::HASH_SIZE];
        ctx.result(&mut out);
        out
    }
}

pub struct Context<H: DigestAlg>(H::DigestContext);

impl<H: DigestAlg> Clone for Context<H> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<H: DigestAlg> Context<H> {
    pub fn new() -> Self {
        Self(H::new())
    }

    pub fn append_data(&mut self, data: &[u8]) {
        H::append_data(&mut self.0, data)
    }

    pub fn finalize(self) -> Digest<H> {
        Digest(H::finalize(self.0))
    }
}

pub struct Digest<H: DigestAlg>(H::DigestData);

impl<H: DigestAlg> Clone for Digest<H> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

macro_rules! define_from_instances {
    ($hash_ty:ty, $hash_size:expr, $bech32_hrp:expr) => {
        impl From<Digest<$hash_ty>> for [u8; $hash_size] {
            fn from(digest: Digest<$hash_ty>) -> Self {
                digest.0
            }
        }
        impl From<[u8; $hash_size]> for Digest<$hash_ty> {
            fn from(bytes: [u8; $hash_size]) -> Self {
                Digest(bytes)
            }
        }
        impl Bech32 for Digest<$hash_ty> {
            const BECH32_HRP: &'static str = $bech32_hrp;

            fn try_from_bech32_str(bech32_str: &str) -> bech32::Result<Self> {
                let bytes = bech32::try_from_bech32_to_bytes::<Self>(bech32_str)?;
                Digest::try_from(&bytes[..]).map_err(bech32::Error::data_invalid)
            }

            fn to_bech32_str(&self) -> String {
                bech32::to_bech32_from_bytes::<Self>(self.as_ref())
            }
        }
    };
}

define_from_instances!(Sha3_256, 32, "sha3");
define_from_instances!(Blake2b256, 32, "blake2b");

impl<H: DigestAlg> PartialEq for Digest<H> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<H: DigestAlg> AsRef<[u8]> for Digest<H> {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl<H: DigestAlg> Hash for Digest<H> {
    fn hash<HA: Hasher>(&self, state: &mut HA) {
        self.0.hash(state)
    }
}

impl<H: DigestAlg> TryFrom<&[u8]> for Digest<H> {
    type Error = Error;
    fn try_from(slice: &[u8]) -> Result<Digest<H>, Self::Error> {
        <H as DigestAlg>::try_from_slice(slice).map(Digest)
    }
}

impl<H: DigestAlg> FromStr for Digest<H> {
    type Err = Error;
    fn from_str(s: &str) -> result::Result<Digest<H>, Self::Err> {
        let bytes = hex::decode(s)?;
        Digest::try_from(&bytes[..])
    }
}

impl<H: DigestAlg> fmt::Display for Digest<H> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", hex::encode(self.as_ref()))
    }
}
impl<H: DigestAlg> fmt::Debug for Digest<H> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(concat!(stringify!($hash_ty), "(0x"))?;
        write!(f, "{}", hex::encode(self.as_ref()))?;
        f.write_str(")")
    }
}
