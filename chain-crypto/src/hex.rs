//! simple implementation of hexadecimal encoding and decoding

use std::{error, fmt};

const ALPHABET: &'static [u8] = b"0123456789abcdef";

pub fn encode<D: AsRef<[u8]>>(input: D) -> String {
    encode_bytes(input.as_ref())
}

fn encode_bytes(input: &[u8]) -> String {
    let mut v = Vec::with_capacity(input.len() * 2);
    for &byte in input.iter() {
        v.push(ALPHABET[(byte >> 4) as usize]);
        v.push(ALPHABET[(byte & 0xf) as usize]);
    }

    unsafe { String::from_utf8_unchecked(v) }
}

/// Errors that may occur during hexadecimal decoding.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum DecodeError {
    /// A character was encountered is not part of the supported
    /// hexadecimal alphabet. Contains the index of the faulty byte.
    InvalidHexChar(usize),
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            DecodeError::InvalidHexChar(idx) => {
                write!(f, "Non-hexadecimal character at byte index {}", idx)
            }
        }
    }
}

impl error::Error for DecodeError {}

pub fn decode<S: AsRef<[u8]>>(input: S) -> Result<Vec<u8>, DecodeError> {
    decode_bytes(input.as_ref())
}

fn decode_bytes(input: &[u8]) -> Result<Vec<u8>, DecodeError> {
    let mut b = Vec::with_capacity(input.len() / 2);
    let mut modulus = 0;
    let mut buf = 0;

    for (idx, byte) in input.iter().enumerate() {
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
                return Err(DecodeError::InvalidHexChar(idx));
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
    fn encode<D: AsRef<[u8]>>(input: D, expected: &str) {
        let encoded = super::encode(input);
        assert_eq!(encoded, expected);
    }
    fn decode<S: AsRef<[u8]>>(expected: &[u8], input: S) {
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

    #[test]
    fn test_bytes() {
        encode(&[1, 2, 3, 4], "01020304");
        decode(&[1, 2, 3, 4], b"01020304");
    }

    #[test]
    fn test_string() {
        encode("1234", "31323334");
        decode(&[1, 2, 3, 4], "01020304");
    }
}
