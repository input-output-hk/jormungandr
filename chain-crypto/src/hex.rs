//! simple implementation of hexadecimal encoding and decoding

const ALPHABET: &'static [u8] = b"0123456789abcdef";

pub fn encode(input: &[u8]) -> String {
    let mut v = Vec::with_capacity(input.len() * 2);
    for &byte in input.iter() {
        v.push(ALPHABET[(byte >> 4) as usize]);
        v.push(ALPHABET[(byte & 0xf) as usize]);
    }

    unsafe { String::from_utf8_unchecked(v) }
}
