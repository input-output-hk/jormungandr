//! Ed25519-BIP32 derivation scheme
//
//! Implementation of the Ed25519-BIP32 paper
//!
//! Supports:
//! * Hard and Soft derivation using 32 bits indices
//! * Derivation Scheme V2
//! * Derivation Scheme V1 (don't use for new code, only for compat)
//!
use cryptoxide::curve25519::{ge_scalarmult_base, sc_reduce, GeP3};
use cryptoxide::digest::Digest;
use cryptoxide::ed25519;
use cryptoxide::ed25519::signature_extended;
use cryptoxide::hmac::Hmac;
use cryptoxide::mac::Mac;
use cryptoxide::sha2::Sha512;

use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::{
    fmt,
    io::{BufRead, Write},
    result,
};

pub const SEED_SIZE: usize = 32;


/// HDWallet errors
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum Error {
    /// the given seed is of invalid size, the parameter is given the given size
    ///
    /// See `SEED_SIZE` for details about the expected size.
    InvalidSeedSize(usize),
    /// the given extended private key is of invalid size. The parameter is the given size.
    ///
    /// See `XPRV_SIZE` for the expected size.
    InvalidXPrvSize(usize),
    /// the given extended public key is of invalid size. The parameter is the given size.
    ///
    /// See `XPUB_SIZE`
    InvalidXPubSize(usize),
    /// the given signature is of invalid size. The parameter is the given size.
    ///
    /// See `SIGNATURE_SIZE` for the expected size.
    InvalidSignatureSize(usize),
    /// The given extended private key is of invalid format for our usage of ED25519.
    ///
    /// This is not a problem of the size, see `Error::InvalidXPrvSize`
    InvalidXPrv(&'static str),
    HexadecimalError(hex::Error),
    ExpectedSoftDerivation,
    InvalidDerivation,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &Error::InvalidSeedSize(sz) => write!(
                f,
                "Invalid Seed Size, expected {} bytes, but received {} bytes.",
                SEED_SIZE, sz
            ),
            &Error::InvalidXPrvSize(sz) => write!(
                f,
                "Invalid XPrv Size, expected {} bytes, but received {} bytes.",
                XPRV_SIZE, sz
            ),
            &Error::InvalidXPubSize(sz) => write!(
                f,
                "Invalid XPub Size, expected {} bytes, but received {} bytes.",
                XPUB_SIZE, sz
            ),
            &Error::InvalidSignatureSize(sz) => write!(
                f,
                "Invalid Signature Size, expected {} bytes, but received {} bytes.",
                SIGNATURE_SIZE, sz
            ),
            &Error::InvalidXPrv(ref err) => write!(f, "Invalid XPrv: {}", err),
            &Error::HexadecimalError(_) => write!(f, "Invalid hexadecimal."),
            &Error::ExpectedSoftDerivation => write!(f, "expected soft derivation"),
            &Error::InvalidDerivation => write!(f, "invalid derivation"),
        }
    }
}
impl From<hex::Error> for Error {
    fn from(e: hex::Error) -> Error {
        Error::HexadecimalError(e)
    }
}
impl ::std::error::Error for Error {
    fn cause(&self) -> Option<&::std::error::Error> {
        match self {
            Error::HexadecimalError(ref err) => Some(err),
            _ => None,
        }
    }
}

pub type Result<T> = result::Result<T, Error>;

/// Seed used to generate the root private key of the HDWallet.
///
#[derive(Debug)]
pub struct Seed([u8; SEED_SIZE]);
impl Seed {
    /// create a Seed by taking ownership of the given array
    ///
    /// ```
    /// use cardano::hdwallet::{Seed, SEED_SIZE};
    ///
    /// let bytes = [0u8;SEED_SIZE];
    /// let seed  = Seed::from_bytes(bytes);
    ///
    /// assert!(seed.as_ref().len() == SEED_SIZE);
    /// ```
    pub fn from_bytes(buf: [u8; SEED_SIZE]) -> Self {
        Seed(buf)
    }

    /// create a Seed by copying the given slice into a new array
    ///
    /// ```
    /// use cardano::hdwallet::{Seed, SEED_SIZE};
    ///
    /// let bytes = [0u8;SEED_SIZE];
    /// let wrong = [0u8;31];
    ///
    /// assert!(Seed::from_slice(&wrong[..]).is_err());
    /// assert!(Seed::from_slice(&bytes[..]).is_ok());
    /// ```
    pub fn from_slice(buf: &[u8]) -> Result<Self> {
        if buf.len() != SEED_SIZE {
            return Err(Error::InvalidSeedSize(buf.len()));
        }
        let mut v = [0u8; SEED_SIZE];
        v[..].clone_from_slice(buf);
        Ok(Seed::from_bytes(v))
    }
}
impl Clone for Seed {
    fn clone(&self) -> Self {
        let mut bytes = [0; SEED_SIZE];
        bytes.copy_from_slice(self.as_ref());
        Seed::from_bytes(bytes)
    }
}
impl AsRef<[u8]> for Seed {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}
impl Drop for Seed {
    fn drop(&mut self) {
        securemem::zero(&mut self.0);
    }
}

/// a signature with an associated type tag
///
#[derive(Clone)]
pub struct Signature<T> {
    bytes: [u8; SIGNATURE_SIZE],
    _phantom: PhantomData<T>,
}
impl<T> Signature<T> {
    pub fn from_bytes(bytes: [u8; SIGNATURE_SIZE]) -> Self {
        Signature {
            bytes: bytes,
            _phantom: PhantomData,
        }
    }

    pub fn from_slice(bytes: &[u8]) -> Result<Self> {
        if bytes.len() != SIGNATURE_SIZE {
            return Err(Error::InvalidSignatureSize(bytes.len()));
        }
        let mut buf = [0u8; SIGNATURE_SIZE];
        buf[..].clone_from_slice(bytes);
        Ok(Self::from_bytes(buf))
    }

    pub fn from_hex(hex: &str) -> Result<Self> {
        let bytes = hex::decode(hex)?;
        Self::from_slice(&bytes)
    }

    pub fn coerce<R>(self) -> Signature<R> {
        Signature::<R>::from_bytes(self.bytes)
    }

    pub fn to_bytes<'a>(&'a self) -> &'a [u8; SIGNATURE_SIZE] {
        &self.bytes
    }
}
impl<T> PartialEq for Signature<T> {
    fn eq(&self, rhs: &Signature<T>) -> bool {
        fixed_time_eq(self.as_ref(), rhs.as_ref())
    }
}
impl<T> Eq for Signature<T> {}
impl<T> fmt::Display for Signature<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", hex::encode(self.as_ref()))
    }
}
impl<T> fmt::Debug for Signature<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", hex::encode(self.as_ref()))
    }
}
impl<T> AsRef<[u8]> for Signature<T> {
    fn as_ref(&self) -> &[u8] {
        &self.bytes
    }
}
impl<T> cbor_event::se::Serialize for Signature<T> {
    fn serialize<'se, W: Write>(
        &self,
        serializer: &'se mut Serializer<W>,
    ) -> cbor_event::Result<&'se mut Serializer<W>> {
        serializer.write_bytes(self.as_ref())
    }
}
impl<T> cbor_event::de::Deserialize for Signature<T> {
    fn deserialize<R: BufRead>(reader: &mut Deserializer<R>) -> cbor_event::Result<Self> {
        let bytes = reader.bytes()?;
        match Signature::from_slice(&bytes) {
            Ok(signature) => Ok(signature),
            Err(Error::InvalidSignatureSize(sz)) => {
                Err(cbor_event::Error::NotEnough(sz, SIGNATURE_SIZE))
            }
            Err(err) => Err(cbor_event::Error::CustomError(format!(
                "unexpected error: {:?}",
                err
            ))),
        }
    }
}

fn mk_ed25519_extended(extended_out: &mut [u8], secret: &[u8]) {
    assert!(extended_out.len() == 64);
    assert!(secret.len() == 32);
    let mut hasher = Sha512::new();
    hasher.input(secret);
    hasher.result(extended_out);
    extended_out[0] &= 248;
    extended_out[31] &= 63;
    extended_out[31] |= 64;
}

#[cfg(test)]
mod tests {
    use super::*;

    const D1: [u8; XPRV_SIZE] = [
        0xf8, 0xa2, 0x92, 0x31, 0xee, 0x38, 0xd6, 0xc5, 0xbf, 0x71, 0x5d, 0x5b, 0xac, 0x21, 0xc7,
        0x50, 0x57, 0x7a, 0xa3, 0x79, 0x8b, 0x22, 0xd7, 0x9d, 0x65, 0xbf, 0x97, 0xd6, 0xfa, 0xde,
        0xa1, 0x5a, 0xdc, 0xd1, 0xee, 0x1a, 0xbd, 0xf7, 0x8b, 0xd4, 0xbe, 0x64, 0x73, 0x1a, 0x12,
        0xde, 0xb9, 0x4d, 0x36, 0x71, 0x78, 0x41, 0x12, 0xeb, 0x6f, 0x36, 0x4b, 0x87, 0x18, 0x51,
        0xfd, 0x1c, 0x9a, 0x24, 0x73, 0x84, 0xdb, 0x9a, 0xd6, 0x00, 0x3b, 0xbd, 0x08, 0xb3, 0xb1,
        0xdd, 0xc0, 0xd0, 0x7a, 0x59, 0x72, 0x93, 0xff, 0x85, 0xe9, 0x61, 0xbf, 0x25, 0x2b, 0x33,
        0x12, 0x62, 0xed, 0xdf, 0xad, 0x0d,
    ];

    const D1_H0: [u8; XPRV_SIZE] = [
        0x60, 0xd3, 0x99, 0xda, 0x83, 0xef, 0x80, 0xd8, 0xd4, 0xf8, 0xd2, 0x23, 0x23, 0x9e, 0xfd,
        0xc2, 0xb8, 0xfe, 0xf3, 0x87, 0xe1, 0xb5, 0x21, 0x91, 0x37, 0xff, 0xb4, 0xe8, 0xfb, 0xde,
        0xa1, 0x5a, 0xdc, 0x93, 0x66, 0xb7, 0xd0, 0x03, 0xaf, 0x37, 0xc1, 0x13, 0x96, 0xde, 0x9a,
        0x83, 0x73, 0x4e, 0x30, 0xe0, 0x5e, 0x85, 0x1e, 0xfa, 0x32, 0x74, 0x5c, 0x9c, 0xd7, 0xb4,
        0x27, 0x12, 0xc8, 0x90, 0x60, 0x87, 0x63, 0x77, 0x0e, 0xdd, 0xf7, 0x72, 0x48, 0xab, 0x65,
        0x29, 0x84, 0xb2, 0x1b, 0x84, 0x97, 0x60, 0xd1, 0xda, 0x74, 0xa6, 0xf5, 0xbd, 0x63, 0x3c,
        0xe4, 0x1a, 0xdc, 0xee, 0xf0, 0x7a,
    ];

    const MSG: &'static [u8] = b"Hello World";

    const D1_H0_SIGNATURE: [u8; 64] = [
        0x90, 0x19, 0x4d, 0x57, 0xcd, 0xe4, 0xfd, 0xad, 0xd0, 0x1e, 0xb7, 0xcf, 0x16, 0x17, 0x80,
        0xc2, 0x77, 0xe1, 0x29, 0xfc, 0x71, 0x35, 0xb9, 0x77, 0x79, 0xa3, 0x26, 0x88, 0x37, 0xe4,
        0xcd, 0x2e, 0x94, 0x44, 0xb9, 0xbb, 0x91, 0xc0, 0xe8, 0x4d, 0x23, 0xbb, 0xa8, 0x70, 0xdf,
        0x3c, 0x4b, 0xda, 0x91, 0xa1, 0x10, 0xef, 0x73, 0x56, 0x38, 0xfa, 0x7a, 0x34, 0xea, 0x20,
        0x46, 0xd4, 0xbe, 0x04,
    ];

    fn compare_xprv(xprv: &[u8], expected_xprv: &[u8]) {
        assert_eq!(
            xprv[64..].to_vec(),
            expected_xprv[64..].to_vec(),
            "chain code"
        );
        assert_eq!(
            xprv[..64].to_vec(),
            expected_xprv[..64].to_vec(),
            "extended key"
        );
    }

    fn seed_xprv_eq(seed: &Seed, expected_xprv: &[u8; XPRV_SIZE]) {
        let xprv = XPrv::generate_from_seed(&seed);
        compare_xprv(xprv.as_ref(), expected_xprv);
    }

    #[test]
    fn seed_cases() {
        let bytes = [
            0xe3, 0x55, 0x24, 0xa5, 0x18, 0x03, 0x4d, 0xdc, 0x11, 0x92, 0xe1, 0xda, 0xcd, 0x32,
            0xc1, 0xed, 0x3e, 0xaa, 0x3c, 0x3b, 0x13, 0x1c, 0x88, 0xed, 0x8e, 0x7e, 0x54, 0xc4,
            0x9a, 0x5d, 0x09, 0x98,
        ];
        let seed = Seed::from_bytes(bytes);
        seed_xprv_eq(&seed, &D1);
    }

    fn derive_xprv_eq(parent_xprv: &XPrv, idx: DerivationIndex, expected_xprv: [u8; 96]) {
        let child_xprv = derive_private(parent_xprv, idx, DerivationScheme::V2);
        compare_xprv(child_xprv.as_ref(), &expected_xprv);
    }

    #[test]
    fn xprv_derive() {
        let prv = XPrv::from_bytes_verified(D1).unwrap();
        derive_xprv_eq(&prv, 0x80000000, D1_H0);
    }

    fn do_sign(xprv: &XPrv, expected_signature: &[u8]) {
        let signature: Signature<Vec<u8>> = xprv.sign(MSG);
        assert_eq!(signature.as_ref(), expected_signature);
    }

    #[test]
    fn xpub_derive_v1_hardened() {
        let derivation_index = 0x1;
        let seed = Seed::from_bytes([0; 32]);
        let prv = XPrv::generate_from_seed(&seed);
        let _ = prv.derive(DerivationScheme::V1, derivation_index);
    }

    #[test]
    fn xpub_derive_v1_soft() {
        let derivation_index = 0x10000000;
        let seed = Seed::from_bytes([0; 32]);
        let prv = XPrv::generate_from_seed(&seed);
        let xpub = prv.public();
        let child_prv = prv.derive(DerivationScheme::V1, derivation_index);
        let child_xpub = xpub.derive(DerivationScheme::V1, derivation_index).unwrap();
        assert_eq!(child_prv.public(), child_xpub);
    }

    #[test]
    fn xpub_derive_v2() {
        let derivation_index = 0x10000000;
        let prv = XPrv::from_bytes_verified(D1).unwrap();
        let xpub = prv.public();
        let child_prv = prv.derive(DerivationScheme::V2, derivation_index);
        let child_xpub = xpub.derive(DerivationScheme::V2, derivation_index).unwrap();
        assert_eq!(child_prv.public(), child_xpub);
    }

    #[test]
    fn xprv_sign() {
        let prv = XPrv::from_bytes_verified(D1_H0).unwrap();
        do_sign(&prv, &D1_H0_SIGNATURE);
    }

    #[test]
    fn normalize_bytes() {
        let entropies = vec![
            super::super::bip::bip39::Entropy::from_slice(&[0; 16]).unwrap(),
            super::super::bip::bip39::Entropy::from_slice(&[0x1f; 20]).unwrap(),
            super::super::bip::bip39::Entropy::from_slice(&[0xda; 24]).unwrap(),
            super::super::bip::bip39::Entropy::from_slice(&[0x2a; 28]).unwrap(),
            super::super::bip::bip39::Entropy::from_slice(&[0xff; 32]).unwrap(),
        ];
        for entropy in entropies {
            let mut bytes = [0; XPRV_SIZE];
            super::super::wallet::keygen::generate_seed(&entropy, b"trezor", &mut bytes);
            let xprv = XPrv::normalize_bytes(bytes);
            let bytes = xprv.0;
            // calling the from_bytes verified to check the xprv
            // is valid
            let _ = XPrv::from_bytes_verified(bytes).unwrap();
        }
    }

    #[test]
    fn unit_derivation_v1() {
        let seed = Seed::from_bytes([
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ]);
        let xprv0 = XPrv::generate_from_seed(&seed);

        let xpub0 = xprv0.public();
        let xpub0_ref = XPub::from_bytes([
            28, 12, 58, 225, 130, 94, 144, 182, 221, 218, 63, 64, 161, 34, 192, 7, 225, 0, 142,
            131, 178, 225, 2, 193, 66, 186, 239, 183, 33, 215, 44, 26, 93, 54, 97, 222, 185, 6, 79,
            45, 14, 3, 254, 133, 214, 128, 112, 178, 254, 51, 180, 145, 96, 89, 101, 142, 40, 172,
            127, 127, 145, 202, 75, 18,
        ]);
        assert_eq!(xpub0_ref, xpub0);

        let xprv1 = xprv0.derive(DerivationScheme::V1, 0x80000000);
        let xpub1 = xprv1.public();
        let xpub1_ref = XPub::from_bytes([
            155, 186, 125, 76, 223, 83, 124, 115, 51, 236, 62, 66, 30, 151, 236, 155, 157, 73, 110,
            160, 25, 204, 222, 170, 46, 185, 166, 187, 220, 65, 18, 182, 194, 224, 222, 91, 65,
            119, 17, 215, 53, 147, 168, 219, 125, 51, 13, 233, 35, 212, 226, 241, 0, 36, 245, 198,
            28, 19, 91, 74, 49, 43, 106, 167,
        ]);

        assert_eq!(xpub1_ref, xpub1);
    }

    #[test]
    fn unit_derivation_v2() {
        use std::str::FromStr;
        let ds = DerivationScheme::V2;

        let root_prv = XPrv::from_str("402b03cd9c8bed9ba9f9bd6cd9c315ce9fcc59c7c25d37c85a36096617e69d418e35cb4a3b737afd007f0688618f21a8831643c0e6c77fc33c06026d2a0fc93832596435e70647d7d98ef102a32ea40319ca8fb6c851d7346d3bd8f9d1492658").unwrap();
        let root_pk  = XPub::from_str("291ea7aa3766cd26a3a8688375aa07b3fed73c13d42543a9f19a48dc8b6bfd0732596435e70647d7d98ef102a32ea40319ca8fb6c851d7346d3bd8f9d1492658").unwrap();

        let expected_pk = root_prv.public();
        assert_eq!(expected_pk, root_pk);

        let child_prv = XPrv::from_str("78164270a17f697b57f172a7ac58cfbb95e007fdcd968c8c6a2468841fe69d4115c846a5d003f7017374d12105c25930a2bf8c386b7be3c470d8226f3cad8b6b7e64c416800883256828efc63567d8842eda422c413f5ff191512dfce7790984").unwrap();
        let expected_child_prv = root_prv
            .derive(ds, 42 | 0x80000000)
            .derive(ds, 3 | 0x80000000)
            .derive(ds, 5);
        assert_eq!(expected_child_prv, child_prv);
    }
}

#[cfg(test)]
#[cfg(feature = "with-bench")]
mod bench {
    use super::*;
    use test;

    #[bench]
    fn derivate_hard_v1(b: &mut test::Bencher) {
        let seed = Seed::from_bytes([0; SEED_SIZE]);
        let sk = XPrv::generate_from_seed(&seed);
        b.iter(|| {
            let _ = sk.derive(DerivationScheme::V1, 0x80000000);
        })
    }
    #[bench]
    fn derivate_hard_v2(b: &mut test::Bencher) {
        let seed = Seed::from_bytes([0; SEED_SIZE]);
        let sk = XPrv::generate_from_seed(&seed);
        b.iter(|| {
            let _ = sk.derive(DerivationScheme::V2, 0x80000000);
        })
    }

    #[bench]
    fn derivate_soft_v1_xprv(b: &mut test::Bencher) {
        let seed = Seed::from_bytes([0; SEED_SIZE]);
        let sk = XPrv::generate_from_seed(&seed);
        b.iter(|| {
            let _ = sk.derive(DerivationScheme::V1, 0);
        })
    }
    #[bench]
    fn derivate_soft_v2_xprv(b: &mut test::Bencher) {
        let seed = Seed::from_bytes([0; SEED_SIZE]);
        let sk = XPrv::generate_from_seed(&seed);
        b.iter(|| {
            let _ = sk.derive(DerivationScheme::V2, 0);
        })
    }
    #[bench]
    fn derivate_soft_v1_xpub(b: &mut test::Bencher) {
        let seed = Seed::from_bytes([0; SEED_SIZE]);
        let sk = XPrv::generate_from_seed(&seed);
        let pk = sk.public();
        b.iter(|| {
            let _ = pk.derive(DerivationScheme::V1, 0);
        })
    }
    #[bench]
    fn derivate_soft_v2_xpub(b: &mut test::Bencher) {
        let seed = Seed::from_bytes([0; SEED_SIZE]);
        let sk = XPrv::generate_from_seed(&seed);
        let pk = sk.public();
        b.iter(|| {
            let _ = pk.derive(DerivationScheme::V2, 0);
        })
    }
}

#[cfg(test)]
mod golden_tests {
    use super::*;
    use bip::bip39;
    use cbor_event;
    use cryptoxide::blake2b::Blake2b;

    use wallet::keygen;

    #[allow(non_snake_case)]
    #[allow(dead_code)]
    struct TestVector {
        /// BIP39 Seed
        seed: &'static [u8],
        /// Wallet's extended signature
        signature: &'static [u8; 64],
        /// Wallet's extended public key
        xPub: &'static [u8; 64],
        /// UTF8 string
        data_to_sign: &'static str,
        /// Derivation Chain code path: list of derivation path.
        path: &'static [u32],
        /// Wallet's derivation schemes: String either "derivation-scheme1" or "derivation-scheme2"
        derivation_scheme: &'static str,
        /// Master Key Derivation: list of derivation path.
        master_key_generation: &'static str,
        /// UTF8 string
        passphrase: &'static str,
        /// BIP39 mnemonic sentence (in English) of 12 BIP39 Enlighs words
        words: &'static str,
    }

    #[derive(Debug, PartialEq, Eq)]
    enum MasterKeyGeneration {
        RetryOld,
        PBKDF,
    }

    impl TestVector {
        fn get_master_key_generation(&self) -> MasterKeyGeneration {
            match self.master_key_generation {
                "retry-old" => MasterKeyGeneration::RetryOld,
                "pbkdf" => MasterKeyGeneration::PBKDF,
                _ => panic!(
                    "Unnown master key generation: {}",
                    self.master_key_generation
                ),
            }
        }
        fn get_derivation(&self) -> DerivationScheme {
            match self.derivation_scheme {
                "derivation-scheme1" => DerivationScheme::V1,
                "derivation-scheme2" => DerivationScheme::V2,
                _ => panic!("Unnown derivation scheme: {}", self.derivation_scheme),
            }
        }
    }

    fn check_derivation(test_index: usize, test: &TestVector) {
        let mkg = test.get_master_key_generation();
        let scheme = test.get_derivation();

        let mut xprv = match mkg {
            MasterKeyGeneration::RetryOld => XPrv::generate_from_daedalus_seed(&test.seed),
            MasterKeyGeneration::PBKDF => {
                let mut master_key_buffer = [0; XPRV_SIZE];
                println!("{:?} {}", &test.seed, test.seed.len());
                let bip39_seed = super::super::bip::bip39::Entropy::from_slice(&test.seed).unwrap();
                keygen::generate_seed(&bip39_seed, b"", &mut master_key_buffer);
                XPrv::normalize_bytes(master_key_buffer)
            }
        };

        for derivation_index in test.path {
            xprv = xprv.derive(scheme, *derivation_index);
        }

        let xpub = xprv.public();
        let ref_xpub = XPub::from_slice(test.xPub).expect("failed to read the xpub from the test");
        assert_eq!(ref_xpub, xpub, "xpub from test {}", test_index);

        let ref_signature: Signature<Vec<u8>> =
            Signature::from_slice(test.signature).expect("retrieve signature from the golden test");
        let signature = xprv.sign(test.data_to_sign.as_bytes());
        assert_eq!(ref_signature, signature, "xpub from test {}", test_index);
    }

    fn check_mnemonics(test_index: usize, test: &TestVector) {
        let mnemonics = bip39::Mnemonics::from_string(&bip39::dictionary::ENGLISH, test.words)
            .expect("retrieve the mnemonics from the string");
        let entropy = bip39::Entropy::from_mnemonics(&mnemonics)
            .expect("retrieve the entropy from the mnemonics");

        if test.get_master_key_generation() == MasterKeyGeneration::PBKDF {
            return;
        }

        let entropy_bytes = cbor_event::Value::Bytes(Vec::from(entropy.as_ref()));
        let entropy_cbor = cbor!(&entropy_bytes).expect("encode entropy in cbor");
        let seed = {
            let mut blake2b = Blake2b::new(32);
            Digest::input(&mut blake2b, &entropy_cbor);
            let mut out = [0; 32];
            Digest::result(&mut blake2b, &mut out);
            Seed::from_bytes(out)
        };
        let seed_ref_hex = hex::encode(&test.seed[2..]);
        let seed_hex = hex::encode(seed.as_ref());

        assert_eq!(seed_ref_hex, seed_hex, "seed from test {}", test_index);
    }

    #[test]
    fn derivation() {
        let mut test_index = 0;
        for test in TEST_VECTORS.iter() {
            check_derivation(test_index, test);
            test_index += 1;
        }
    }

    #[test]
    fn mnemonics() {
        let mut test_index = 0;
        for test in TEST_VECTORS.iter() {
            check_mnemonics(test_index, test);
            test_index += 1;
        }
    }

    const TEST_VECTORS: [TestVector; 26] = [
        TestVector {
            data_to_sign: "Hello World",
            path: &[],
            derivation_scheme: "derivation-scheme1",
            master_key_generation: "retry-old",
            passphrase: "",
            words: "ring crime symptom enough erupt lady behave ramp apart settle citizen junk",
            seed: &[
                88, 32, 46, 212, 199, 29, 145, 188, 104, 199, 181, 15, 238, 181, 188, 122, 120, 95,
                232, 132, 221, 10, 237, 220, 224, 41, 223, 61, 97, 44, 211, 104, 15, 211,
            ],
            signature: &[
                69, 177, 167, 95, 227, 17, 158, 19, 198, 246, 10, 185, 186, 103, 75, 66, 249, 70,
                253, 197, 88, 224, 124, 131, 223, 160, 117, 28, 46, 186, 105, 199, 147, 49, 189,
                138, 74, 151, 86, 98, 178, 54, 40, 164, 56, 160, 235, 167, 99, 103, 228, 76, 18,
                202, 145, 179, 158, 197, 144, 99, 248, 96, 241, 13,
            ],
            xPub: &[
                100, 178, 15, 160, 130, 179, 20, 61, 107, 94, 237, 66, 198, 239, 99, 249, 149, 153,
                208, 136, 138, 254, 6, 6, 32, 171, 193, 179, 25, 147, 95, 225, 115, 159, 75, 60,
                172, 164, 201, 173, 79, 205, 75, 220, 46, 244, 44, 134, 1, 175, 141, 105, 70, 153,
                158, 248, 94, 246, 174, 132, 246, 110, 114, 235,
            ],
        },
        TestVector {
            data_to_sign: "Hello World",
            path: &[2147483648],
            derivation_scheme: "derivation-scheme1",
            master_key_generation: "retry-old",
            passphrase: "",
            words: "ring crime symptom enough erupt lady behave ramp apart settle citizen junk",
            seed: &[
                88, 32, 46, 212, 199, 29, 145, 188, 104, 199, 181, 15, 238, 181, 188, 122, 120, 95,
                232, 132, 221, 10, 237, 220, 224, 41, 223, 61, 97, 44, 211, 104, 15, 211,
            ],
            signature: &[
                242, 201, 23, 23, 130, 231, 223, 118, 101, 18, 106, 197, 69, 174, 83, 176, 89, 100,
                176, 22, 5, 54, 239, 219, 84, 94, 36, 96, 219, 190, 194, 177, 158, 198, 179, 56,
                184, 241, 191, 77, 254, 233, 67, 96, 237, 2, 75, 17, 94, 55, 177, 215, 230, 243,
                249, 174, 75, 235, 121, 83, 148, 40, 86, 15,
            ],
            xPub: &[
                149, 187, 130, 255, 213, 112, 119, 22, 188, 101, 23, 10, 180, 232, 218, 254, 237,
                144, 251, 224, 206, 146, 88, 113, 59, 119, 81, 233, 98, 217, 49, 223, 103, 85, 203,
                130, 232, 146, 214, 97, 76, 0, 122, 94, 251, 206, 178, 29, 149, 165, 36, 78, 38,
                157, 14, 32, 107, 72, 185, 164, 149, 57, 11, 3,
            ],
        },
        TestVector {
            data_to_sign: "Hello World",
            path: &[2147483649],
            derivation_scheme: "derivation-scheme1",
            master_key_generation: "retry-old",
            passphrase: "",
            words: "ring crime symptom enough erupt lady behave ramp apart settle citizen junk",
            seed: &[
                88, 32, 46, 212, 199, 29, 145, 188, 104, 199, 181, 15, 238, 181, 188, 122, 120, 95,
                232, 132, 221, 10, 237, 220, 224, 41, 223, 61, 97, 44, 211, 104, 15, 211,
            ],
            signature: &[
                43, 161, 67, 154, 230, 72, 167, 232, 218, 124, 154, 177, 238, 109, 169, 79, 212,
                235, 227, 122, 189, 9, 120, 48, 110, 143, 186, 42, 250, 143, 17, 26, 136, 169, 147,
                219, 240, 8, 190, 218, 233, 22, 127, 79, 104, 64, 158, 76, 157, 218, 240, 44, 186,
                18, 65, 132, 71, 177, 132, 137, 7, 173, 128, 15,
            ],
            xPub: &[
                121, 252, 129, 84, 85, 75, 151, 228, 197, 110, 242, 249, 219, 180, 193, 66, 31,
                241, 149, 9, 104, 137, 49, 161, 233, 100, 189, 165, 222, 192, 241, 159, 71, 162,
                66, 113, 59, 209, 134, 8, 35, 17, 71, 192, 102, 182, 8, 59, 252, 30, 144, 102, 254,
                201, 246, 33, 132, 76, 132, 254, 214, 34, 138, 52,
            ],
        },
        TestVector {
            data_to_sign: "Hello World",
            path: &[2147483648, 2147483649],
            derivation_scheme: "derivation-scheme1",
            master_key_generation: "retry-old",
            passphrase: "",
            words: "ring crime symptom enough erupt lady behave ramp apart settle citizen junk",
            seed: &[
                88, 32, 46, 212, 199, 29, 145, 188, 104, 199, 181, 15, 238, 181, 188, 122, 120, 95,
                232, 132, 221, 10, 237, 220, 224, 41, 223, 61, 97, 44, 211, 104, 15, 211,
            ],
            signature: &[
                12, 211, 79, 132, 224, 210, 252, 177, 128, 11, 219, 14, 134, 155, 144, 65, 52, 153,
                85, 206, 214, 106, 237, 190, 107, 218, 24, 126, 190, 141, 54, 166, 42, 5, 179, 150,
                71, 233, 47, 204, 66, 170, 122, 115, 104, 23, 66, 64, 175, 186, 8, 184, 200, 31,
                152, 26, 34, 249, 66, 214, 189, 120, 22, 2,
            ],
            xPub: &[
                220, 144, 124, 124, 6, 230, 49, 78, 237, 217, 225, 140, 159, 108, 111, 156, 196,
                226, 5, 251, 28, 112, 218, 96, 130, 52, 195, 25, 241, 247, 176, 214, 214, 121, 132,
                145, 185, 250, 70, 18, 55, 10, 229, 239, 60, 98, 58, 11, 104, 114, 243, 173, 143,
                38, 151, 8, 133, 250, 103, 200, 59, 220, 66, 94,
            ],
        },
        TestVector {
            data_to_sign: "Hello World",
            path: &[2147483648, 2147483649, 2147483650],
            derivation_scheme: "derivation-scheme1",
            master_key_generation: "retry-old",
            passphrase: "",
            words: "ring crime symptom enough erupt lady behave ramp apart settle citizen junk",
            seed: &[
                88, 32, 46, 212, 199, 29, 145, 188, 104, 199, 181, 15, 238, 181, 188, 122, 120, 95,
                232, 132, 221, 10, 237, 220, 224, 41, 223, 61, 97, 44, 211, 104, 15, 211,
            ],
            signature: &[
                228, 31, 115, 219, 47, 141, 40, 150, 166, 135, 128, 43, 43, 231, 107, 124, 171,
                183, 61, 251, 180, 137, 20, 148, 136, 58, 12, 189, 155, 187, 158, 95, 157, 62, 20,
                210, 208, 176, 108, 102, 116, 51, 53, 8, 73, 109, 182, 96, 147, 103, 55, 192, 239,
                217, 81, 21, 20, 20, 125, 172, 121, 250, 73, 5,
            ],
            xPub: &[
                131, 151, 117, 164, 24, 118, 227, 40, 152, 106, 162, 97, 104, 149, 139, 186, 17,
                118, 230, 120, 25, 179, 87, 238, 168, 74, 252, 234, 184, 177, 219, 120, 65, 105,
                162, 163, 46, 54, 24, 169, 3, 233, 48, 189, 26, 113, 48, 51, 163, 143, 146, 56,
                144, 147, 64, 131, 148, 226, 154, 195, 122, 23, 82, 234,
            ],
        },
        TestVector {
            data_to_sign: "Hello World",
            path: &[2147483648, 2147483649, 2147483650, 2147483650],
            derivation_scheme: "derivation-scheme1",
            master_key_generation: "retry-old",
            passphrase: "",
            words: "ring crime symptom enough erupt lady behave ramp apart settle citizen junk",
            seed: &[
                88, 32, 46, 212, 199, 29, 145, 188, 104, 199, 181, 15, 238, 181, 188, 122, 120, 95,
                232, 132, 221, 10, 237, 220, 224, 41, 223, 61, 97, 44, 211, 104, 15, 211,
            ],
            signature: &[
                99, 16, 21, 53, 124, 238, 48, 81, 17, 107, 76, 47, 244, 209, 197, 190, 177, 59,
                110, 80, 35, 99, 90, 161, 238, 176, 86, 60, 173, 240, 212, 251, 193, 11, 213, 227,
                27, 74, 66, 32, 198, 120, 117, 85, 140, 65, 181, 204, 3, 40, 16, 74, 227, 156, 199,
                255, 32, 255, 12, 43, 218, 89, 137, 6,
            ],
            xPub: &[
                117, 235, 141, 25, 126, 200, 98, 124, 133, 175, 136, 230, 106, 161, 228, 144, 101,
                221, 138, 201, 142, 216, 153, 29, 181, 46, 206, 1, 99, 93, 251, 118, 58, 233, 201,
                154, 89, 37, 203, 162, 220, 241, 33, 186, 243, 160, 37, 79, 61, 234, 35, 193, 41,
                249, 235, 112, 168, 167, 232, 137, 124, 81, 153, 186,
            ],
        },
        TestVector {
            data_to_sign: "Hello World",
            path: &[2147483648, 2147483649, 2147483650, 2147483650, 3147483648],
            derivation_scheme: "derivation-scheme1",
            master_key_generation: "retry-old",
            passphrase: "",
            words: "ring crime symptom enough erupt lady behave ramp apart settle citizen junk",
            seed: &[
                88, 32, 46, 212, 199, 29, 145, 188, 104, 199, 181, 15, 238, 181, 188, 122, 120, 95,
                232, 132, 221, 10, 237, 220, 224, 41, 223, 61, 97, 44, 211, 104, 15, 211,
            ],
            signature: &[
                29, 225, 210, 117, 66, 139, 169, 73, 26, 67, 60, 212, 115, 205, 7, 108, 2, 127, 97,
                231, 168, 181, 57, 29, 249, 222, 165, 203, 75, 200, 141, 138, 87, 176, 149, 144,
                106, 48, 177, 62, 104, 37, 152, 81, 168, 221, 63, 87, 182, 240, 255, 163, 122, 93,
                63, 252, 23, 18, 64, 242, 212, 4, 249, 1,
            ],
            xPub: &[
                5, 136, 88, 156, 217, 181, 29, 252, 2, 140, 242, 37, 103, 64, 105, 203, 229, 46,
                14, 112, 222, 176, 45, 196, 91, 121, 178, 110, 227, 84, 139, 0, 21, 196, 80, 184,
                109, 215, 221, 131, 179, 25, 81, 217, 238, 3, 235, 26, 121, 37, 22, 29, 129, 123,
                213, 23, 198, 156, 240, 158, 54, 113, 241, 202,
            ],
        },
        TestVector {
            data_to_sign: "Hello World",
            path: &[2147483648, 2147483649, 2147483650, 2147483650, 3147483648],
            derivation_scheme: "derivation-scheme2",
            master_key_generation: "retry-old",
            passphrase: "",
            words: "ring crime symptom enough erupt lady behave ramp apart settle citizen junk",
            seed: &[
                88, 32, 46, 212, 199, 29, 145, 188, 104, 199, 181, 15, 238, 181, 188, 122, 120, 95,
                232, 132, 221, 10, 237, 220, 224, 41, 223, 61, 97, 44, 211, 104, 15, 211,
            ],
            signature: &[
                6, 89, 180, 164, 55, 100, 90, 197, 228, 99, 111, 18, 9, 34, 98, 119, 122, 151, 211,
                67, 121, 168, 12, 35, 60, 186, 191, 232, 1, 90, 221, 180, 147, 194, 151, 220, 180,
                115, 9, 65, 61, 181, 80, 124, 45, 104, 112, 202, 209, 158, 142, 19, 187, 217, 107,
                181, 211, 51, 193, 184, 222, 61, 57, 13,
            ],
            xPub: &[
                92, 231, 23, 39, 87, 99, 212, 40, 3, 64, 177, 124, 34, 102, 71, 224, 202, 42, 227,
                84, 191, 18, 48, 46, 205, 171, 79, 104, 214, 15, 117, 189, 144, 116, 171, 55, 6,
                15, 138, 48, 131, 1, 110, 111, 55, 85, 222, 88, 1, 111, 32, 159, 106, 113, 3, 214,
                59, 31, 128, 197, 63, 153, 219, 153,
            ],
        },
        TestVector {
            data_to_sign: "Hello World",
            path: &[2147483648, 2147483649],
            derivation_scheme: "derivation-scheme2",
            master_key_generation: "retry-old",
            passphrase: "",
            words: "ring crime symptom enough erupt lady behave ramp apart settle citizen junk",
            seed: &[
                88, 32, 46, 212, 199, 29, 145, 188, 104, 199, 181, 15, 238, 181, 188, 122, 120, 95,
                232, 132, 221, 10, 237, 220, 224, 41, 223, 61, 97, 44, 211, 104, 15, 211,
            ],
            signature: &[
                57, 187, 18, 182, 103, 242, 87, 134, 98, 255, 102, 125, 155, 187, 145, 12, 221,
                198, 44, 73, 21, 53, 159, 133, 170, 109, 6, 135, 86, 239, 14, 75, 99, 242, 18, 34,
                17, 88, 99, 17, 248, 105, 73, 160, 76, 197, 10, 251, 220, 189, 88, 169, 235, 183,
                255, 197, 61, 164, 15, 79, 80, 156, 255, 11,
            ],
            xPub: &[
                105, 115, 241, 204, 85, 27, 87, 42, 250, 27, 209, 180, 179, 170, 176, 182, 52, 39,
                101, 41, 243, 111, 218, 111, 7, 1, 149, 145, 7, 127, 95, 161, 245, 169, 113, 47,
                193, 23, 102, 163, 253, 216, 157, 247, 104, 159, 78, 137, 30, 230, 64, 44, 230, 44,
                37, 146, 6, 156, 209, 38, 9, 200, 169, 28,
            ],
        },
        TestVector {
            data_to_sign: "Data",
            path: &[2147483648, 2147483649, 24, 2000],
            derivation_scheme: "derivation-scheme2",
            master_key_generation: "retry-old",
            passphrase: "",
            words: "ring crime symptom enough erupt lady behave ramp apart settle citizen junk",
            seed: &[
                88, 32, 46, 212, 199, 29, 145, 188, 104, 199, 181, 15, 238, 181, 188, 122, 120, 95,
                232, 132, 221, 10, 237, 220, 224, 41, 223, 61, 97, 44, 211, 104, 15, 211,
            ],
            signature: &[
                181, 219, 221, 11, 145, 249, 5, 65, 41, 224, 207, 65, 95, 81, 185, 150, 126, 153,
                51, 193, 131, 62, 144, 138, 149, 65, 52, 121, 184, 243, 57, 234, 58, 147, 249, 249,
                227, 29, 201, 172, 12, 86, 26, 55, 29, 99, 133, 159, 196, 186, 1, 236, 14, 31, 232,
                228, 85, 204, 166, 150, 63, 68, 13, 1,
            ],
            xPub: &[
                227, 18, 13, 24, 35, 120, 212, 160, 131, 244, 47, 144, 169, 196, 186, 2, 114, 189,
                10, 99, 41, 227, 137, 106, 177, 148, 140, 253, 169, 185, 4, 32, 60, 0, 11, 80, 63,
                132, 79, 227, 236, 34, 198, 198, 91, 205, 196, 203, 69, 170, 186, 152, 165, 202,
                252, 5, 171, 37, 176, 67, 96, 73, 66, 19,
            ],
        },
        TestVector {
            data_to_sign: "Hello World",
            path: &[2147483648, 2147483649, 24, 2147485648],
            derivation_scheme: "derivation-scheme2",
            master_key_generation: "retry-old",
            passphrase: "",
            words: "ring crime symptom enough erupt lady behave ramp apart settle citizen junk",
            seed: &[
                88, 32, 46, 212, 199, 29, 145, 188, 104, 199, 181, 15, 238, 181, 188, 122, 120, 95,
                232, 132, 221, 10, 237, 220, 224, 41, 223, 61, 97, 44, 211, 104, 15, 211,
            ],
            signature: &[
                53, 131, 252, 13, 24, 244, 25, 23, 4, 7, 248, 138, 199, 199, 4, 201, 78, 48, 209,
                29, 105, 131, 38, 131, 26, 64, 43, 231, 65, 164, 182, 236, 92, 70, 78, 252, 57,
                172, 210, 33, 58, 67, 63, 210, 79, 203, 33, 33, 153, 129, 42, 238, 233, 26, 42,
                236, 217, 4, 60, 212, 215, 191, 152, 10,
            ],
            xPub: &[
                53, 86, 55, 241, 36, 158, 11, 182, 196, 84, 9, 114, 137, 131, 98, 242, 71, 217,
                242, 185, 244, 171, 117, 222, 13, 148, 237, 136, 0, 81, 74, 27, 117, 134, 67, 112,
                95, 234, 81, 191, 233, 49, 109, 141, 108, 209, 49, 91, 65, 79, 231, 171, 37, 21,
                148, 156, 184, 138, 204, 197, 236, 203, 150, 228,
            ],
        },
        TestVector {
            data_to_sign: "Hello World",
            path: &[],
            derivation_scheme: "derivation-scheme1",
            master_key_generation: "retry-old",
            passphrase: "",
            words: "leaf immune metal phrase river cool domain snow year below result three",
            seed: &[
                88, 32, 125, 97, 13, 1, 77, 51, 0, 85, 70, 52, 144, 202, 73, 13, 215, 83, 233, 244,
                211, 149, 250, 162, 176, 35, 122, 23, 245, 216, 254, 190, 172, 68,
            ],
            signature: &[
                206, 16, 29, 142, 121, 242, 95, 165, 43, 154, 79, 144, 190, 78, 191, 253, 124, 100,
                58, 186, 156, 96, 188, 51, 93, 19, 117, 96, 145, 135, 201, 60, 161, 14, 7, 202, 81,
                14, 176, 22, 97, 177, 181, 227, 132, 59, 91, 181, 176, 46, 248, 135, 2, 250, 4,
                129, 179, 217, 110, 229, 37, 251, 4, 5,
            ],
            xPub: &[
                199, 220, 27, 150, 169, 206, 224, 8, 2, 183, 91, 246, 133, 197, 39, 0, 95, 195,
                223, 210, 10, 43, 92, 114, 121, 254, 13, 146, 234, 81, 191, 3, 208, 233, 236, 170,
                180, 87, 200, 222, 165, 86, 187, 46, 244, 62, 197, 156, 201, 67, 177, 42, 219, 57,
                201, 211, 141, 77, 144, 86, 59, 144, 20, 167,
            ],
        },
        TestVector {
            data_to_sign: "Hello World",
            path: &[],
            derivation_scheme: "derivation-scheme1",
            master_key_generation: "pbkdf",
            passphrase: "",
            words: "ring crime symptom enough erupt lady behave ramp apart settle citizen junk",
            seed: &[
                186, 6, 115, 114, 37, 116, 206, 249, 5, 29, 139, 10, 88, 140, 165, 60,
            ],
            signature: &[
                3, 208, 116, 96, 23, 210, 73, 63, 228, 241, 179, 8, 65, 228, 157, 8, 83, 159, 242,
                159, 134, 127, 249, 13, 241, 196, 163, 16, 156, 3, 28, 147, 222, 73, 161, 48, 75,
                230, 116, 151, 240, 58, 138, 111, 128, 5, 165, 88, 49, 158, 167, 104, 66, 87, 93,
                243, 75, 190, 147, 148, 63, 198, 152, 10,
            ],
            xPub: &[
                154, 29, 4, 128, 139, 76, 6, 130, 129, 105, 97, 207, 102, 110, 130, 167, 253, 53,
                148, 150, 88, 171, 165, 53, 76, 81, 126, 204, 241, 42, 172, 180, 175, 251, 195, 37,
                217, 2, 124, 15, 45, 159, 146, 91, 29, 207, 108, 18, 191, 92, 29, 208, 137, 4, 71,
                64, 102, 164, 242, 192, 13, 181, 97, 115,
            ],
        },
        TestVector {
            data_to_sign: "Hello World",
            path: &[2147483648],
            derivation_scheme: "derivation-scheme1",
            master_key_generation: "pbkdf",
            passphrase: "",
            words: "ring crime symptom enough erupt lady behave ramp apart settle citizen junk",
            seed: &[
                186, 6, 115, 114, 37, 116, 206, 249, 5, 29, 139, 10, 88, 140, 165, 60,
            ],
            signature: &[
                209, 13, 32, 178, 130, 16, 120, 15, 42, 189, 103, 9, 183, 219, 133, 151, 155, 119,
                213, 20, 34, 53, 246, 227, 202, 165, 25, 220, 11, 143, 66, 132, 18, 128, 47, 224,
                221, 197, 214, 161, 162, 159, 241, 20, 181, 166, 206, 30, 75, 31, 177, 84, 190, 12,
                216, 17, 98, 38, 85, 143, 100, 254, 68, 13,
            ],
            xPub: &[
                99, 61, 12, 18, 216, 229, 250, 118, 85, 142, 65, 42, 109, 217, 13, 66, 249, 210,
                77, 62, 187, 6, 123, 152, 115, 216, 141, 204, 52, 210, 211, 211, 75, 181, 2, 230,
                75, 116, 103, 92, 105, 137, 171, 35, 166, 236, 12, 92, 251, 74, 190, 131, 114, 241,
                199, 111, 13, 220, 250, 139, 163, 129, 148, 206,
            ],
        },
        TestVector {
            data_to_sign: "Hello World",
            path: &[],
            derivation_scheme: "derivation-scheme2",
            master_key_generation: "pbkdf",
            passphrase: "",
            words: "ring crime symptom enough erupt lady behave ramp apart settle citizen junk",
            seed: &[
                186, 6, 115, 114, 37, 116, 206, 249, 5, 29, 139, 10, 88, 140, 165, 60,
            ],
            signature: &[
                3, 208, 116, 96, 23, 210, 73, 63, 228, 241, 179, 8, 65, 228, 157, 8, 83, 159, 242,
                159, 134, 127, 249, 13, 241, 196, 163, 16, 156, 3, 28, 147, 222, 73, 161, 48, 75,
                230, 116, 151, 240, 58, 138, 111, 128, 5, 165, 88, 49, 158, 167, 104, 66, 87, 93,
                243, 75, 190, 147, 148, 63, 198, 152, 10,
            ],
            xPub: &[
                154, 29, 4, 128, 139, 76, 6, 130, 129, 105, 97, 207, 102, 110, 130, 167, 253, 53,
                148, 150, 88, 171, 165, 53, 76, 81, 126, 204, 241, 42, 172, 180, 175, 251, 195, 37,
                217, 2, 124, 15, 45, 159, 146, 91, 29, 207, 108, 18, 191, 92, 29, 208, 137, 4, 71,
                64, 102, 164, 242, 192, 13, 181, 97, 115,
            ],
        },
        TestVector {
            data_to_sign: "Hello World",
            path: &[2147483649],
            derivation_scheme: "derivation-scheme1",
            master_key_generation: "pbkdf",
            passphrase: "",
            words: "ring crime symptom enough erupt lady behave ramp apart settle citizen junk",
            seed: &[
                186, 6, 115, 114, 37, 116, 206, 249, 5, 29, 139, 10, 88, 140, 165, 60,
            ],
            signature: &[
                246, 6, 84, 122, 84, 225, 156, 49, 85, 214, 100, 79, 50, 110, 240, 234, 96, 21,
                117, 218, 191, 88, 144, 121, 204, 41, 250, 62, 64, 249, 105, 250, 82, 148, 240,
                111, 14, 150, 54, 124, 95, 60, 197, 66, 14, 242, 181, 19, 237, 222, 142, 125, 251,
                70, 70, 75, 239, 213, 188, 75, 191, 134, 150, 0,
            ],
            xPub: &[
                71, 245, 20, 113, 225, 112, 118, 142, 20, 56, 180, 197, 12, 127, 94, 148, 229, 27,
                100, 1, 165, 123, 171, 228, 250, 17, 225, 4, 65, 228, 196, 253, 118, 183, 141, 194,
                142, 224, 74, 49, 106, 171, 156, 81, 209, 93, 223, 177, 7, 113, 85, 16, 16, 176,
                96, 95, 67, 73, 141, 226, 114, 224, 250, 25,
            ],
        },
        TestVector {
            data_to_sign: "Hello World",
            path: &[2147483648, 2147483649],
            derivation_scheme: "derivation-scheme1",
            master_key_generation: "pbkdf",
            passphrase: "",
            words: "ring crime symptom enough erupt lady behave ramp apart settle citizen junk",
            seed: &[
                186, 6, 115, 114, 37, 116, 206, 249, 5, 29, 139, 10, 88, 140, 165, 60,
            ],
            signature: &[
                9, 184, 205, 94, 165, 153, 29, 20, 6, 64, 164, 87, 255, 154, 219, 248, 56, 124,
                152, 190, 121, 164, 50, 67, 101, 125, 232, 95, 108, 191, 252, 248, 59, 249, 80,
                127, 160, 23, 4, 248, 87, 95, 29, 187, 44, 21, 101, 178, 216, 18, 202, 204, 80,
                199, 252, 198, 201, 25, 63, 121, 67, 128, 21, 6,
            ],
            xPub: &[
                40, 217, 160, 7, 135, 157, 147, 223, 143, 19, 168, 220, 36, 65, 93, 121, 85, 143,
                195, 152, 202, 16, 107, 54, 149, 169, 67, 40, 118, 80, 42, 219, 210, 243, 174, 31,
                102, 109, 240, 63, 94, 205, 15, 190, 126, 236, 239, 193, 134, 47, 150, 150, 163,
                81, 59, 80, 224, 37, 188, 208, 103, 139, 30, 108,
            ],
        },
        TestVector {
            data_to_sign: "Hello World",
            path: &[2147483648, 2147483649, 2147483650],
            derivation_scheme: "derivation-scheme1",
            master_key_generation: "pbkdf",
            passphrase: "",
            words: "ring crime symptom enough erupt lady behave ramp apart settle citizen junk",
            seed: &[
                186, 6, 115, 114, 37, 116, 206, 249, 5, 29, 139, 10, 88, 140, 165, 60,
            ],
            signature: &[
                43, 243, 233, 94, 18, 43, 0, 64, 194, 8, 211, 85, 66, 88, 77, 63, 215, 172, 52,
                183, 94, 91, 240, 172, 30, 121, 127, 109, 159, 145, 220, 22, 71, 111, 62, 78, 120,
                255, 127, 74, 6, 201, 159, 133, 59, 174, 225, 212, 132, 95, 47, 204, 5, 118, 35,
                62, 24, 152, 31, 242, 164, 41, 224, 4,
            ],
            xPub: &[
                179, 83, 196, 107, 149, 85, 94, 164, 232, 142, 92, 64, 174, 60, 32, 199, 42, 167,
                79, 168, 220, 57, 80, 227, 111, 76, 212, 213, 15, 172, 135, 139, 195, 162, 102,
                224, 108, 206, 141, 193, 134, 133, 115, 158, 219, 235, 247, 135, 238, 152, 173,
                244, 67, 20, 143, 244, 220, 194, 170, 189, 210, 32, 34, 13,
            ],
        },
        TestVector {
            data_to_sign: "Hello World",
            path: &[2147483648, 2147483649, 2147483650, 2147483650],
            derivation_scheme: "derivation-scheme1",
            master_key_generation: "pbkdf",
            passphrase: "",
            words: "ring crime symptom enough erupt lady behave ramp apart settle citizen junk",
            seed: &[
                186, 6, 115, 114, 37, 116, 206, 249, 5, 29, 139, 10, 88, 140, 165, 60,
            ],
            signature: &[
                245, 228, 223, 76, 193, 122, 78, 108, 150, 210, 61, 104, 133, 208, 105, 58, 16,
                121, 124, 149, 57, 94, 218, 225, 243, 63, 199, 123, 1, 3, 134, 54, 44, 137, 116,
                169, 151, 170, 154, 2, 130, 23, 26, 122, 13, 200, 169, 100, 216, 18, 147, 22, 246,
                182, 117, 249, 35, 224, 200, 95, 83, 46, 215, 11,
            ],
            xPub: &[
                138, 174, 22, 49, 78, 210, 63, 112, 88, 1, 32, 209, 152, 227, 43, 104, 65, 255, 97,
                105, 190, 80, 68, 99, 7, 227, 238, 225, 230, 191, 34, 248, 177, 132, 252, 17, 181,
                105, 71, 119, 147, 232, 83, 235, 30, 23, 129, 78, 147, 15, 64, 92, 236, 168, 246,
                154, 63, 234, 80, 85, 165, 120, 184, 199,
            ],
        },
        TestVector {
            data_to_sign: "Hello World",
            path: &[2147483648, 2147483649, 2147483650, 2147483650, 3147483648],
            derivation_scheme: "derivation-scheme1",
            master_key_generation: "pbkdf",
            passphrase: "",
            words: "ring crime symptom enough erupt lady behave ramp apart settle citizen junk",
            seed: &[
                186, 6, 115, 114, 37, 116, 206, 249, 5, 29, 139, 10, 88, 140, 165, 60,
            ],
            signature: &[
                140, 110, 43, 66, 222, 89, 53, 232, 37, 45, 57, 187, 107, 23, 71, 183, 120, 41,
                219, 254, 35, 62, 249, 105, 158, 71, 163, 9, 75, 148, 63, 127, 162, 171, 207, 152,
                242, 76, 230, 240, 254, 226, 35, 64, 196, 95, 143, 187, 32, 64, 16, 250, 235, 20,
                131, 162, 189, 63, 117, 125, 17, 82, 188, 7,
            ],
            xPub: &[
                0, 103, 250, 100, 213, 47, 119, 241, 26, 111, 234, 171, 144, 162, 174, 225, 20,
                228, 10, 203, 171, 110, 25, 73, 131, 163, 79, 124, 233, 158, 184, 104, 39, 206, 96,
                249, 187, 143, 216, 240, 187, 220, 85, 138, 6, 165, 2, 40, 117, 91, 252, 186, 36,
                1, 8, 194, 109, 203, 103, 161, 146, 135, 141, 252,
            ],
        },
        TestVector {
            data_to_sign: "Hello World",
            path: &[2147483648, 2147483649, 2147483650, 2147483650, 3147483648],
            derivation_scheme: "derivation-scheme2",
            master_key_generation: "pbkdf",
            passphrase: "",
            words: "ring crime symptom enough erupt lady behave ramp apart settle citizen junk",
            seed: &[
                186, 6, 115, 114, 37, 116, 206, 249, 5, 29, 139, 10, 88, 140, 165, 60,
            ],
            signature: &[
                155, 221, 224, 59, 175, 3, 29, 219, 136, 130, 154, 9, 0, 48, 181, 22, 250, 175,
                108, 147, 69, 14, 255, 227, 3, 251, 182, 169, 250, 163, 148, 136, 103, 129, 16, 51,
                60, 10, 206, 248, 93, 181, 224, 92, 198, 201, 84, 181, 83, 157, 51, 136, 158, 33,
                117, 252, 40, 13, 231, 232, 188, 81, 104, 8,
            ],
            xPub: &[
                20, 134, 5, 190, 84, 88, 87, 115, 180, 75, 168, 126, 121, 38, 81, 73, 174, 68, 76,
                76, 195, 124, 177, 248, 219, 140, 8, 72, 47, 186, 41, 59, 255, 119, 192, 141, 55,
                71, 28, 29, 76, 237, 211, 250, 226, 100, 44, 0, 147, 36, 217, 113, 36, 146, 239,
                199, 77, 237, 171, 9, 201, 191, 151, 60,
            ],
        },
        TestVector {
            data_to_sign: "Hello World",
            path: &[2147483648, 2147483649],
            derivation_scheme: "derivation-scheme2",
            master_key_generation: "pbkdf",
            passphrase: "",
            words: "ring crime symptom enough erupt lady behave ramp apart settle citizen junk",
            seed: &[
                186, 6, 115, 114, 37, 116, 206, 249, 5, 29, 139, 10, 88, 140, 165, 60,
            ],
            signature: &[
                90, 44, 53, 169, 237, 186, 191, 31, 163, 95, 210, 158, 152, 129, 189, 177, 79, 1,
                17, 175, 16, 246, 254, 169, 201, 169, 181, 213, 113, 246, 213, 141, 35, 166, 111,
                25, 58, 47, 243, 196, 123, 53, 134, 107, 255, 197, 227, 254, 45, 162, 212, 60, 100,
                9, 215, 102, 247, 253, 40, 122, 97, 1, 1, 13,
            ],
            xPub: &[
                170, 172, 165, 231, 173, 198, 154, 3, 239, 31, 92, 1, 126, 208, 40, 121, 232, 202,
                135, 29, 240, 40, 70, 30, 217, 191, 25, 251, 143, 161, 80, 56, 180, 12, 68, 223,
                217, 190, 8, 89, 27, 98, 190, 127, 153, 145, 200, 95, 129, 45, 129, 150, 146, 127,
                60, 130, 77, 159, 203, 23, 210, 117, 8, 158,
            ],
        },
        TestVector {
            data_to_sign: "Data",
            path: &[2147483648, 2147483649, 24, 2000],
            derivation_scheme: "derivation-scheme2",
            master_key_generation: "pbkdf",
            passphrase: "",
            words: "ring crime symptom enough erupt lady behave ramp apart settle citizen junk",
            seed: &[
                186, 6, 115, 114, 37, 116, 206, 249, 5, 29, 139, 10, 88, 140, 165, 60,
            ],
            signature: &[
                25, 54, 93, 210, 173, 255, 186, 108, 183, 145, 11, 152, 13, 177, 138, 0, 14, 32,
                166, 166, 234, 16, 183, 186, 13, 8, 45, 239, 246, 24, 192, 197, 34, 191, 209, 177,
                223, 95, 175, 15, 54, 55, 155, 23, 243, 223, 243, 123, 62, 115, 8, 73, 240, 95, 78,
                99, 51, 209, 73, 136, 125, 120, 57, 9,
            ],
            xPub: &[
                152, 188, 243, 148, 252, 51, 245, 208, 166, 225, 53, 193, 201, 52, 198, 156, 50,
                123, 195, 201, 29, 208, 45, 179, 242, 239, 44, 128, 91, 186, 171, 16, 52, 47, 233,
                181, 179, 187, 21, 67, 33, 213, 38, 245, 200, 5, 56, 50, 75, 182, 121, 100, 129,
                124, 204, 12, 234, 34, 122, 73, 80, 37, 176, 7,
            ],
        },
        TestVector {
            data_to_sign: "Hello World",
            path: &[2147483648, 2147483649, 24, 2147485648],
            derivation_scheme: "derivation-scheme2",
            master_key_generation: "pbkdf",
            passphrase: "",
            words: "ring crime symptom enough erupt lady behave ramp apart settle citizen junk",
            seed: &[
                186, 6, 115, 114, 37, 116, 206, 249, 5, 29, 139, 10, 88, 140, 165, 60,
            ],
            signature: &[
                211, 41, 138, 25, 22, 68, 54, 246, 184, 230, 4, 87, 232, 85, 179, 215, 200, 217,
                213, 115, 223, 78, 127, 21, 244, 50, 13, 108, 87, 90, 57, 34, 13, 16, 214, 244,
                157, 137, 160, 123, 96, 36, 18, 248, 38, 183, 176, 138, 89, 158, 141, 82, 249, 205,
                32, 182, 55, 50, 161, 0, 8, 190, 12, 12,
            ],
            xPub: &[
                105, 104, 158, 194, 147, 46, 164, 205, 139, 94, 166, 215, 60, 242, 165, 180, 244,
                254, 1, 99, 189, 41, 246, 108, 147, 61, 160, 170, 249, 57, 196, 62, 195, 101, 31,
                62, 223, 235, 229, 35, 32, 37, 240, 241, 177, 137, 246, 7, 169, 235, 131, 254, 209,
                194, 101, 163, 77, 247, 3, 114, 38, 180, 248, 228,
            ],
        },
        TestVector {
            data_to_sign: "Hello World",
            path: &[2147483648],
            derivation_scheme: "derivation-scheme1",
            master_key_generation: "retry-old",
            passphrase: "",
            words: "leaf immune metal phrase river cool domain snow year below result three",
            seed: &[
                88, 32, 125, 97, 13, 1, 77, 51, 0, 85, 70, 52, 144, 202, 73, 13, 215, 83, 233, 244,
                211, 149, 250, 162, 176, 35, 122, 23, 245, 216, 254, 190, 172, 68,
            ],
            signature: &[
                69, 183, 75, 168, 122, 123, 22, 8, 13, 113, 83, 197, 82, 35, 26, 46, 225, 183, 153,
                146, 176, 96, 24, 200, 139, 245, 80, 251, 189, 177, 205, 87, 198, 45, 108, 23, 113,
                68, 52, 31, 94, 184, 199, 123, 1, 243, 114, 206, 181, 229, 94, 155, 22, 142, 105,
                250, 73, 77, 2, 197, 192, 53, 67, 6,
            ],
            // xPriv: & [ 14, 11, 245, 52, 7, 46, 253, 178, 231, 57, 165, 216, 33, 39, 172, 179, 151, 177, 26, 221, 31, 195, 45, 86, 202, 242, 8, 104, 178, 160, 10, 8, 175, 176, 132, 22, 150, 190, 214, 17, 212, 174, 28, 204, 254, 38, 190, 56, 165, 214, 223, 164, 240, 85, 23, 252, 105, 178, 17, 62, 211, 53, 193, 31, 22, 78, 242, 8, 99, 42, 109, 131, 55, 79, 197, 182, 219, 254, 28, 158, 166, 222, 30, 198, 116, 34, 155, 216, 123, 206, 210, 38, 236, 42, 245, 1, 200, 74, 50, 232, 107, 238, 130, 102, 131, 239, 62, 8, 4, 205, 95, 43, 81, 182, 112, 247, 114, 85, 195, 197, 129, 173, 212, 120, 157, 128, 156, 63],
            xPub: &[
                22, 78, 242, 8, 99, 42, 109, 131, 55, 79, 197, 182, 219, 254, 28, 158, 166, 222,
                30, 198, 116, 34, 155, 216, 123, 206, 210, 38, 236, 42, 245, 1, 200, 74, 50, 232,
                107, 238, 130, 102, 131, 239, 62, 8, 4, 205, 95, 43, 81, 182, 112, 247, 114, 85,
                195, 197, 129, 173, 212, 120, 157, 128, 156, 63,
            ],
        },
        TestVector {
            data_to_sign: "Hello World",
            path: &[2147483648, 2147483648],
            derivation_scheme: "derivation-scheme1",
            master_key_generation: "retry-old",
            passphrase: "",
            words: "leaf immune metal phrase river cool domain snow year below result three",
            seed: &[
                88, 32, 125, 97, 13, 1, 77, 51, 0, 85, 70, 52, 144, 202, 73, 13, 215, 83, 233, 244,
                211, 149, 250, 162, 176, 35, 122, 23, 245, 216, 254, 190, 172, 68,
            ],
            signature: &[
                13, 208, 10, 118, 61, 241, 62, 187, 68, 2, 96, 15, 171, 8, 102, 166, 179, 139, 201,
                25, 71, 71, 79, 129, 96, 89, 226, 42, 66, 1, 66, 107, 229, 83, 149, 131, 34, 32,
                149, 167, 69, 173, 15, 243, 96, 114, 28, 241, 67, 125, 76, 194, 123, 122, 163, 37,
                128, 45, 109, 140, 249, 7, 126, 4,
            ],
            // xPriv: & [ 69, 160, 184, 196, 99, 247, 209, 218, 88, 170, 159, 18, 121, 102, 201, 227, 15, 218, 138, 69, 200, 43, 54, 198, 66, 235, 225, 200, 42, 145, 51, 8, 202, 164, 17, 3, 254, 189, 173, 206, 151, 111, 213, 29, 138, 21, 88, 235, 8, 239, 162, 116, 148, 230, 190, 201, 94, 107, 170, 37, 244, 126, 194, 39, 173, 84, 27, 134, 66, 198, 63, 6, 174, 99, 11, 118, 133, 226, 104, 43, 38, 66, 35, 85, 9, 180, 106, 212, 178, 55, 173, 214, 167, 136, 254, 127, 159, 116, 94, 167, 137, 90, 206, 219, 4, 95, 183, 240, 108, 111, 107, 66, 21, 138, 134, 209, 203, 224, 229, 238, 128, 35, 176, 238, 17, 51, 148, 199],
            xPub: &[
                173, 84, 27, 134, 66, 198, 63, 6, 174, 99, 11, 118, 133, 226, 104, 43, 38, 66, 35,
                85, 9, 180, 106, 212, 178, 55, 173, 214, 167, 136, 254, 127, 159, 116, 94, 167,
                137, 90, 206, 219, 4, 95, 183, 240, 108, 111, 107, 66, 21, 138, 134, 209, 203, 224,
                229, 238, 128, 35, 176, 238, 17, 51, 148, 199,
            ],
        },
    ];
}
