use super::hash;
use chain_crypto::{Ed25519Extended, SecretKey, SecretKeyError};
use image::{DynamicImage, ImageBuffer, Luma};
use qrcodegen::{QrCode, QrCodeEcc};
use std::fs::File;
use std::io::{self, prelude::*};
use std::path::Path;
use symmetric_cipher::Error as SymmetricCipherError;
use thiserror::Error;

const MODULE_SIZE: i32 = 8;
const QR_CODE_BORDER: i32 = 4 * 8;

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
    QrDecodeError(#[from] QrDecodeError),
    #[error("failed to decode hex")]
    HexDecodeError(#[from] hex::FromHexError),
    #[error("failed to decode hex")]
    QrCodeHashError(#[from] super::hash::Error),
}

#[derive(Error, Debug)]
pub enum QrDecodeError {
    #[error("couldn't decode QR code")]
    DecodeError(#[from] quircs::DecodeError),
    #[error("couldn't extract QR code")]
    ExtractError(#[from] quircs::ExtractError),
    #[error("QR code payload is not valid uf8")]
    NonUtf8Payload,
}

impl KeyQrCode {
    pub fn generate(key: SecretKey<Ed25519Extended>, password: &[u8]) -> Self {
        let enc_hex = hash::generate(key, password);
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

        let inner_size_in_pixels = qr.size() * MODULE_SIZE;
        let size = QR_CODE_BORDER * 2 + inner_size_in_pixels;

        let mut img = ImageBuffer::from_pixel(size as u32, size as u32, Luma([255u8]));

        for x in 0..qr.size() {
            for y in 0..qr.size() {
                if qr.get_module(x, y) {
                    // draw a block square of module_size * module_size
                    for i in 0..MODULE_SIZE {
                        for j in 0..MODULE_SIZE {
                            img.put_pixel(
                                (x * MODULE_SIZE + i + QR_CODE_BORDER) as u32,
                                (y * MODULE_SIZE + j + QR_CODE_BORDER) as u32,
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
    ) -> Result<Vec<SecretKey<Ed25519Extended>>, KeyQrCodeError> {
        let mut decoder = quircs::Quirc::default();

        let img = img.into_luma8();

        let codes = decoder.identify(img.width() as usize, img.height() as usize, &img);

        codes
            .map(|code| -> Result<_, KeyQrCodeError> {
                let decoded = code
                    .map_err(QrDecodeError::ExtractError)
                    .and_then(|c| c.decode().map_err(QrDecodeError::DecodeError))?;

                // TODO: I actually don't know if this can fail
                let h = std::str::from_utf8(&decoded.payload)
                    .map_err(|_| QrDecodeError::NonUtf8Payload)?;
                hash::decode(h, password).map_err(Into::into)
            })
            .collect()
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
            KeyQrCode::decode(DynamicImage::ImageLuma8(img), PASSWORD).unwrap()[0]
                .clone()
                .leak_secret()
                .as_ref()
        );
    }
}
