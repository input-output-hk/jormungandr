mod common;
pub mod v1;
pub mod v2;

use cryptoxide::curve25519::{ge_scalarmult_base, GeP3};
use cryptoxide::hmac::Hmac;
use cryptoxide::mac::Mac;
use cryptoxide::sha2::Sha512;

use super::key::{mk_public_key, mk_xprv, mk_xpub, XPrv, XPub, XPRV_SIZE, XPUB_SIZE};
pub use common::{DerivationIndex, DerivationScheme, DerivationType};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DerivationError {
    InvalidAddition,
    ExpectedSoftDerivation,
}

fn add_256bits(x: &[u8], y: &[u8], scheme: DerivationScheme) -> [u8; 32] {
    match scheme {
        DerivationScheme::V1 => v1::add_256bits_v1(x, y),
        DerivationScheme::V2 => v2::add_256bits_v2(x, y),
    }
}

fn add_28_mul8(x: &[u8], y: &[u8], scheme: DerivationScheme) -> [u8; 32] {
    match scheme {
        DerivationScheme::V1 => v1::add_28_mul8_v1(x, y),
        DerivationScheme::V2 => v2::add_28_mul8_v2(x, y),
    }
}

fn serialize_index(i: u32, derivation_scheme: DerivationScheme) -> [u8; 4] {
    match derivation_scheme {
        DerivationScheme::V1 => v1::be32(i),
        DerivationScheme::V2 => v2::le32(i),
    }
}

pub fn private(xprv: &XPrv, index: DerivationIndex, scheme: DerivationScheme) -> XPrv {
    /*
     * If so (hardened child):
     *    let Z = HMAC-SHA512(Key = cpar, Data = 0x00 || ser256(left(kpar)) || ser32(i)).
     *    let I = HMAC-SHA512(Key = cpar, Data = 0x01 || ser256(left(kpar)) || ser32(i)).
     * If not (normal child):
     *    let Z = HMAC-SHA512(Key = cpar, Data = 0x02 || serP(point(kpar)) || ser32(i)).
     *    let I = HMAC-SHA512(Key = cpar, Data = 0x03 || serP(point(kpar)) || ser32(i)).
     **/

    let ekey = &xprv.as_ref()[0..64];
    let kl = &ekey[0..32];
    let kr = &ekey[32..64];
    let chaincode = &xprv.as_ref()[64..96];

    let mut zmac = Hmac::new(Sha512::new(), &chaincode);
    let mut imac = Hmac::new(Sha512::new(), &chaincode);
    let seri = serialize_index(index, scheme);
    match DerivationType::from_index(index) {
        DerivationType::Soft(_) => {
            let pk = mk_public_key(ekey);
            zmac.input(&[0x2]);
            zmac.input(&pk);
            zmac.input(&seri);
            imac.input(&[0x3]);
            imac.input(&pk);
            imac.input(&seri);
        }
        DerivationType::Hard(_) => {
            zmac.input(&[0x0]);
            zmac.input(ekey);
            zmac.input(&seri);
            imac.input(&[0x1]);
            imac.input(ekey);
            imac.input(&seri);
        }
    };

    let mut zout = [0u8; 64];
    zmac.raw_result(&mut zout);
    let zl = &zout[0..32];
    let zr = &zout[32..64];

    // left = kl + 8 * trunc28(zl)
    let left = add_28_mul8(kl, zl, scheme);
    // right = zr + kr
    let right = add_256bits(kr, zr, scheme);

    let mut iout = [0u8; 64];
    imac.raw_result(&mut iout);
    let cc = &iout[32..];

    let mut out = [0u8; XPRV_SIZE];
    mk_xprv(&mut out, &left, &right, cc);

    imac.reset();
    zmac.reset();

    XPrv::from_bytes(out)
}

fn point_of_trunc28_mul8(sk: &[u8], scheme: DerivationScheme) -> [u8; 32] {
    assert!(sk.len() == 32);
    let copy = add_28_mul8(&[0u8; 32], sk, scheme);
    let a = ge_scalarmult_base(&copy);
    a.to_bytes()
}

fn point_plus(p1: &[u8], p2: &[u8]) -> Result<[u8; 32], DerivationError> {
    let a = match GeP3::from_bytes_negate_vartime(p1) {
        Some(g) => g,
        None => {
            return Err(DerivationError::InvalidAddition);
        }
    };
    let b = match GeP3::from_bytes_negate_vartime(p2) {
        Some(g) => g,
        None => {
            return Err(DerivationError::InvalidAddition);
        }
    };
    let r = a + b.to_cached();
    let mut r = r.to_p2().to_bytes();
    r[31] ^= 0x80;
    Ok(r)
}

pub fn public(
    xpub: &XPub,
    index: DerivationIndex,
    scheme: DerivationScheme,
) -> Result<XPub, DerivationError> {
    let pk = &xpub.as_ref()[0..32];
    let chaincode = &xpub.as_ref()[32..64];

    let mut zmac = Hmac::new(Sha512::new(), &chaincode);
    let mut imac = Hmac::new(Sha512::new(), &chaincode);
    let seri = serialize_index(index, scheme);
    match DerivationType::from_index(index) {
        DerivationType::Soft(_) => {
            zmac.input(&[0x2]);
            zmac.input(&pk);
            zmac.input(&seri);
            imac.input(&[0x3]);
            imac.input(&pk);
            imac.input(&seri);
        }
        DerivationType::Hard(_) => {
            return Err(DerivationError::ExpectedSoftDerivation);
        }
    };

    let mut zout = [0u8; 64];
    zmac.raw_result(&mut zout);
    let zl = &zout[0..32];
    let _zr = &zout[32..64];

    // left = kl + 8 * trunc28(zl)
    let left = point_plus(pk, &point_of_trunc28_mul8(zl, scheme))?;

    let mut iout = [0u8; 64];
    imac.raw_result(&mut iout);
    let cc = &iout[32..];

    let mut out = [0u8; XPUB_SIZE];
    mk_xpub(&mut out, &left, cc);

    imac.reset();
    zmac.reset();

    Ok(XPub::from_bytes(out))
}
