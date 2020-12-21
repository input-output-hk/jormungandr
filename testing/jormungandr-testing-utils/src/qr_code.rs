use chain_crypto::{Ed25519Extended, SecretKey, SecretKeyError};
use image::{DynamicImage, ImageBuffer, Luma};
use qrcodegen::{QrCode, QrCodeEcc};
use std::fs::File;
use std::io::{self, prelude::*};
use std::path::Path;
use symmetric_cipher::{decrypt, encrypt, Error as SymmetricCipherError};
use thiserror::Error;

const QR_CODE_BORDER: i32 = 8;

pub struct KeyQrCode {
    inner: QrCode,
}

#[derive(Error, Debug)]
pub enum KeyQrCodeError {
    #[error("encryption-decryption protocol error")]
    SymmetricCipher(#[from] SymmetricCipherError),
    #[error("io error")]
    Io(#[from] io::Error),
    #[error("invalid secret key")]
    SecretKey(#[from] SecretKeyError),
    #[error("couldn't decode QR code")]
    QrDecodeError,
    #[error("failed to decode hex")]
    HexDecodeError(#[from] hex::FromHexError),
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
        let inner = QrCode::encode_text(&enc_hex, QrCodeEcc::High).unwrap();

        KeyQrCode { inner }
    }

    pub fn write_svg(&self, path: impl AsRef<Path>) -> Result<(), KeyQrCodeError> {
        let mut out = File::create(path)?;
        let svg = self.inner.to_svg_string(QR_CODE_BORDER);
        out.write_all(svg.as_bytes())?;
        out.flush()?;
        Ok(())
    }

    pub fn to_img(&self) -> ImageBuffer<Luma<u8>, Vec<u8>> {
        let qr = &self.inner;

        let module_size = 8;
        let pixel_size = qr.size() * module_size;
        let size = QR_CODE_BORDER * 2 + pixel_size;

        let mut img = ImageBuffer::from_pixel(size as u32, size as u32, Luma([255u8]));

        for x in 0..qr.size() {
            for y in 0..qr.size() {
                if qr.get_module(x, y) {
                    // draw a block square of module_size * module_size
                    for i in 0..module_size {
                        for j in 0..module_size {
                            img.put_pixel(
                                (x * module_size + i + QR_CODE_BORDER) as u32,
                                (y * module_size + j + QR_CODE_BORDER) as u32,
                                Luma([0u8]),
                            );
                        }
                    }
                }
            }
        }

        img
    }

    pub fn decode(
        img: DynamicImage,
        password: &[u8],
    ) -> Result<SecretKey<Ed25519Extended>, KeyQrCodeError> {
        let decoder = bardecoder::default_decoder();

        let results = decoder.decode(&img);

        let bytes = hex::decode(
            &results[0]
                .as_ref()
                .map_err(|_| KeyQrCodeError::QrDecodeError)?,
        )?;

        SecretKey::from_binary(decrypt(password, bytes)?.as_ref()).map_err(KeyQrCodeError::from)
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

    #[test]
    #[ignore]
    fn encode_decode() {
        const PASSWORD: &[u8] = &[1, 2, 3, 4];
        let sk = SecretKey::generate(rand::thread_rng());
        let qr = KeyQrCode::generate(sk.clone(), PASSWORD);
        let img = qr.to_img();
        // img.save("qr.png").unwrap();
        assert_eq!(
            sk.leak_secret().as_ref(),
            KeyQrCode::decode(DynamicImage::ImageLuma8(img), PASSWORD)
                .unwrap()
                .leak_secret()
                .as_ref()
        );
    }
}
