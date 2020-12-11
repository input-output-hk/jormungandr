use chain_crypto::{Ed25519Extended, SecretKey};

use cryptoxide::chacha20poly1305::ChaCha20Poly1305;
use cryptoxide::hmac::Hmac;
use cryptoxide::pbkdf2::pbkdf2;
use cryptoxide::sha2::Sha512;
use qrcodegen::{QrCode, QrCodeEcc};
use rand::prelude::*;

use std::fs::File;
use std::io::{self, prelude::*};
use std::iter;
use std::path::Path;

const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 12;
const TAG_LEN: usize = 16;
const PASSWORD_DERIVATION_ITERATIONS: u32 = 12_983;
const PROTO_VERSION: u8 = 1;
const QR_CODE_BORDER: i32 = 2;

pub struct KeyQrCode {
    inner: QrCode,
}

impl KeyQrCode {
    pub fn generate(key: SecretKey<Ed25519Extended>, password: &[u8]) -> Self {
        let secret = key.leak_secret();
        let enc = encrypt(secret.as_ref(), password);
        // Using binary would make the QR codes more compact and probably less
        // prone to scanning errors.
        let enc_hex = hex::encode(enc);
        let inner = QrCode::encode_text(&enc_hex, QrCodeEcc::Medium).unwrap();
        KeyQrCode { inner }
    }

    pub fn write_svg(&self, path: impl AsRef<Path>) -> io::Result<()> {
        let mut out = File::create(path)?;
        let svg = self.inner.to_svg_string(QR_CODE_BORDER);
        out.write_all(svg.as_bytes())?;
        out.flush()?;
        Ok(())
    }
}

fn encrypt(input: &[u8], password: &[u8]) -> Vec<u8> {
    let mut rng = rand::thread_rng();
    let salt = rng.gen::<[u8; SALT_LEN]>();
    let nonce = rng.gen::<[u8; NONCE_LEN]>();
    let mut aead = ChaCha20Poly1305::new(&pass_to_key(password, &salt), &nonce, &[]);
    let mut enc = Vec::with_capacity(1 + SALT_LEN + NONCE_LEN + input.len() + TAG_LEN);
    enc.push(PROTO_VERSION);
    enc.extend_from_slice(&salt);
    enc.extend_from_slice(&nonce);
    let ciphertext_offset = enc.len();
    enc.extend(iter::repeat(0).take(input.len() + TAG_LEN));
    let (ciphertext, tag) = enc[ciphertext_offset..].split_at_mut(input.len());
    aead.encrypt(input, ciphertext, tag);
    enc
}

fn pass_to_key(password: &[u8], salt: &[u8]) -> [u8; 32] {
    let mut hmac = Hmac::new(Sha512::new(), password);
    let mut output = [0; 32];
    pbkdf2(&mut hmac, salt, PASSWORD_DERIVATION_ITERATIONS, &mut output);
    output
}
