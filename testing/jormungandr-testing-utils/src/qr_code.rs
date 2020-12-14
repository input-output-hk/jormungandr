use chain_crypto::{Ed25519Extended, SecretKey};
use qrcodegen::{QrCode, QrCodeEcc};

use std::fs::File;
use std::io::{self, prelude::*};
use std::path::Path;
use symmetric_cipher::encrypt;

const QR_CODE_BORDER: i32 = 2;

pub struct KeyQrCode {
    inner: QrCode,
}

impl KeyQrCode {
    pub fn generate(key: SecretKey<Ed25519Extended>, password: &[u8]) -> Self {
        let secret = key.leak_secret();
        let rng = rand::thread_rng();
        // this won't fail because we already know it's an ed25519extended key,
        // so it is safe to unwrap
        let enc = encrypt(password, secret.as_ref(), rng).unwrap();
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

#[cfg(test)]
mod tests {
    use super::*;

    // TODO: Improve into an integration test using a temporary directory.
    // Leaving here as an example.
    #[test]
    #[ignore]
    fn generate_svg() {
        const PASSWORD: &[u8] = &[1, 2, 3, 4];
        let sk = SecretKey::generate(rand::thread_rng());
        let qr = KeyQrCode::generate(sk, PASSWORD);
        qr.write_svg("qr-code.svg").unwrap();
    }
}
