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
    /// Length of hex string is not even. Last character doesn't have a pair to encode a whole byte.
    UnevenHexLength(usize),
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            DecodeError::InvalidHexChar(idx) => {
                write!(f, "Non-hexadecimal character at byte index {}", idx)
            }
            DecodeError::UnevenHexLength(len) => {
                write!(f, "Hex has uneven number of characters {}", len)
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
    match modulus {
        0 => Ok(b),
        _ => Err(DecodeError::UnevenHexLength(b.len() * 2 + 1)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_decode(input: impl AsRef<[u8]>, expected: impl AsRef<[u8]>) {
        let result = decode(&input);

        let input_str = format!("{:?}", input.as_ref());
        let actual = result.expect(&format!("Failed to decode '{}'", input_str));
        assert_eq!(
            actual,
            expected.as_ref(),
            "Decoded invalid data from '{}'",
            input_str
        );
    }

    fn refute_decode(input: impl AsRef<[u8]>, expected: DecodeError) {
        let result = decode(&input);

        let input_str = format!("{:?}", input.as_ref());
        let actual = result.expect_err(&format!("Did not fail to decode '{}'", input_str));
        assert_eq!(
            actual, expected,
            "Invalid error when decoding '{}'",
            input_str
        );
    }

    #[test]
    fn test_decode() {
        assert_decode("01020304", [0x01, 0x02, 0x03, 0x04]);
        assert_decode(b"01020304", [0x01, 0x02, 0x03, 0x04]);
        assert_decode("0123456789", [0x01, 0x23, 0x45, 0x67, 0x89]);
        assert_decode("abcdef", [0xAB, 0xCD, 0xEF]);
        assert_decode("ABCDEF", [0xAB, 0xCD, 0xEF]);
        assert_decode(" 0\t\r102 \n", [0x01, 0x02]);
        refute_decode("010x0304", DecodeError::InvalidHexChar(3));
        refute_decode("0102030", DecodeError::UnevenHexLength(7));
    }

    fn assert_encode(input: impl AsRef<[u8]>, expected: &str) {
        let actual = encode(&input);

        assert_eq!(
            actual,
            expected,
            "Invalid output for input {:?}",
            input.as_ref()
        );
    }

    #[test]
    fn test_encode() {
        assert_encode([0x01, 0x02, 0x03, 0x04], "01020304");
        assert_encode([0x01, 0x23, 0x45, 0x67, 0x89], "0123456789");
        assert_encode([0xAB, 0xCD, 0xEF], "abcdef");
    }
}
