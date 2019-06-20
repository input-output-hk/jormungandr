//! simple implementation of hexadecimal encoding and decoding
//!
//! # Example
//!
//! ```
//! use hex::{Error, encode, decode};
//!
//! let example = b"some bytes";
//!
//! assert!(example.as_ref() == decode(&encode(example)).unwrap().as_slice());
//! ```
//!
use std::{fmt, result};

const ALPHABET: &'static [u8] = b"0123456789abcdef";

/// hexadecimal encoding/decoding potential errors
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
#[cfg_attr(feature = "generic-serialization", derive(Serialize, Deserialize))]
pub enum Error {
    /// error when a given character is not part of the supported
    /// hexadecimal alphabet. Contains the index of the faulty byte
    UnknownSymbol(usize),
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &Error::UnknownSymbol(idx) => write!(f, "Unknown symbol at byte index {}", idx),
        }
    }
}
impl ::std::error::Error for Error {}

pub type Result<T> = result::Result<T, Error>;

/// encode bytes into an hexadecimal string
///
///  # Example
///
/// ```
/// use hex::{Error, encode};
///
/// let example = b"some bytes";
///
/// assert_eq!("736f6d65206279746573", encode(example));
/// ```
pub fn encode(input: &[u8]) -> String {
    let mut v = Vec::with_capacity(input.len() * 2);
    for &byte in input.iter() {
        v.push(ALPHABET[(byte >> 4) as usize]);
        v.push(ALPHABET[(byte & 0xf) as usize]);
    }

    unsafe { String::from_utf8_unchecked(v) }
}

/// decode the given hexadecimal string
///
///  # Example
///
/// ```
/// use hex::{Error, decode};
///
/// let example = r"736f6d65206279746573";
///
/// assert!(decode(example).is_ok());
/// ```
pub fn decode(input: &str) -> Result<Vec<u8>> {
    let mut b = Vec::with_capacity(input.len() / 2);
    let mut modulus = 0;
    let mut buf = 0;

    for (idx, byte) in input.bytes().enumerate() {
        buf <<= 4;

        match byte {
            b'A'...b'F' => buf |= byte - b'A' + 10,
            b'a'...b'f' => buf |= byte - b'a' + 10,
            b'0'...b'9' => buf |= byte - b'0',
            b' ' | b'\r' | b'\n' | b'\t' => {
                buf >>= 4;
                continue;
            }
            _ => {
                return Err(Error::UnknownSymbol(idx));
            }
        }

        modulus += 1;
        if modulus == 2 {
            modulus = 0;
            b.push(buf);
        }
    }

    Ok(b)
}

#[cfg(test)]
mod tests {
    fn encode(input: &[u8], expected: &str) {
        let encoded = super::encode(input);
        assert_eq!(encoded, expected);
    }
    fn decode(expected: &[u8], input: &str) {
        let decoded = super::decode(input).unwrap();
        assert_eq!(decoded.as_slice(), expected);
    }

    #[test]
    fn test_vector_1() {
        encode(&[1, 2, 3, 4], "01020304");
        decode(&[1, 2, 3, 4], "01020304");
    }

    #[test]
    fn test_vector_2() {
        encode(&[0xff, 0x0f, 0xff, 0xff], "ff0fffff");
        decode(&[0xff, 0x0f, 0xff, 0xff], "ff0fffff");
    }
}
