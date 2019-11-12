use bech32::{Bech32, FromBase32 as _, ToBase32 as _};
use chain_core::mempack::{ReadBuf, ReadError, Readable};
use chain_core::property;
use chain_impl_mockchain::certificate;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{fmt, str::FromStr};
use typed_bytes::ByteBuilder;

pub const SIGNED_CERTIFICATE_HRP: &str = "signedcert";
pub const CERTIFICATE_HRP: &str = "cert";

#[derive(Debug, Clone)]
pub struct Certificate(pub certificate::Certificate);

#[derive(Debug, Clone)]
pub struct SignedCertificate(pub certificate::SignedCertificate);

impl PartialEq for SignedCertificate {
    fn eq(&self, other: &Self) -> bool {
        use property::Serialize as _;
        match (self.serialize_as_vec(), other.serialize_as_vec()) {
            (Ok(a), Ok(b)) => a == b,
            _ => false,
        }
    }
}

impl SignedCertificate {
    pub fn strip_auth(self) -> Certificate {
        match self.0 {
            certificate::SignedCertificate::StakeDelegation(c, _) => {
                Certificate(certificate::Certificate::StakeDelegation(c))
            }
            certificate::SignedCertificate::OwnerStakeDelegation(c, _) => {
                Certificate(certificate::Certificate::OwnerStakeDelegation(c))
            }
            certificate::SignedCertificate::PoolRegistration(c, _) => {
                Certificate(certificate::Certificate::PoolRegistration(c))
            }
            certificate::SignedCertificate::PoolRetirement(c, _) => {
                Certificate(certificate::Certificate::PoolRetirement(c))
            }
            certificate::SignedCertificate::PoolUpdate(c, _) => {
                Certificate(certificate::Certificate::PoolUpdate(c))
            }
        }
    }
}

impl property::Serialize for Certificate {
    type Error = std::io::Error;
    fn serialize<W: std::io::Write>(&self, mut writer: W) -> Result<(), Self::Error> {
        match &self.0 {
            certificate::Certificate::StakeDelegation(c) => {
                writer.write_all(&[1])?;
                c.serialize(&mut writer)?;
            }
            certificate::Certificate::OwnerStakeDelegation(c) => {
                writer.write_all(&[2])?;
                c.serialize(&mut writer)?;
            }
            certificate::Certificate::PoolRegistration(c) => {
                writer.write_all(&[3])?;
                writer.write_all(c.serialize().as_slice())?;
            }
            certificate::Certificate::PoolRetirement(c) => {
                writer.write_all(&[4])?;
                writer.write_all(c.serialize().as_slice())?;
            }
            certificate::Certificate::PoolUpdate(c) => {
                writer.write_all(&[5])?;
                writer.write_all(c.serialize().as_slice())?;
            }
        };
        Ok(())
    }
}

impl Readable for Certificate {
    fn read<'a>(buf: &mut ReadBuf<'a>) -> Result<Self, ReadError> {
        match buf.get_u8()? {
            1 => {
                let cert = certificate::StakeDelegation::read(buf)?;
                Ok(Certificate(certificate::Certificate::StakeDelegation(cert)))
            }
            2 => {
                let cert = certificate::OwnerStakeDelegation::read(buf)?;
                Ok(Certificate(certificate::Certificate::OwnerStakeDelegation(
                    cert,
                )))
            }
            3 => {
                let cert = certificate::PoolRegistration::read(buf)?;
                Ok(Certificate(certificate::Certificate::PoolRegistration(
                    cert,
                )))
            }
            4 => {
                let cert = certificate::PoolRetirement::read(buf)?;
                Ok(Certificate(certificate::Certificate::PoolRetirement(cert)))
            }
            5 => {
                let cert = certificate::PoolUpdate::read(buf)?;
                Ok(Certificate(certificate::Certificate::PoolUpdate(cert)))
            }
            t => Err(ReadError::UnknownTag(t as u32))?,
        }
    }
}

impl property::Serialize for SignedCertificate {
    type Error = std::io::Error;
    fn serialize<W: std::io::Write>(&self, mut writer: W) -> Result<(), Self::Error> {
        match &self.0 {
            certificate::SignedCertificate::StakeDelegation(c, a) => {
                writer.write_all(&[1])?;
                c.serialize(&mut writer)?;
                writer.write_all(a.serialize_in(ByteBuilder::new()).finalize().as_slice())?;
            }
            certificate::SignedCertificate::OwnerStakeDelegation(c, ()) => {
                writer.write_all(&[2])?;
                c.serialize(&mut writer)?;
            }
            certificate::SignedCertificate::PoolRegistration(c, a) => {
                writer.write_all(&[3])?;
                writer.write_all(c.serialize().as_slice())?;
                writer.write_all(a.serialize_in(ByteBuilder::new()).finalize().as_slice())?;
            }
            certificate::SignedCertificate::PoolRetirement(c, a) => {
                writer.write_all(&[4])?;
                writer.write_all(c.serialize().as_slice())?;
                writer.write_all(a.serialize_in(ByteBuilder::new()).finalize().as_slice())?;
            }
            certificate::SignedCertificate::PoolUpdate(c, a) => {
                writer.write_all(&[5])?;
                writer.write_all(c.serialize().as_slice())?;
                writer.write_all(a.serialize_in(ByteBuilder::new()).finalize().as_slice())?;
            }
        };
        Ok(())
    }
}

impl Readable for SignedCertificate {
    fn read<'a>(buf: &mut ReadBuf<'a>) -> Result<Self, ReadError> {
        match buf.get_u8()? {
            1 => {
                let cert = certificate::StakeDelegation::read(buf)?;
                let auth = Readable::read(buf)?;
                Ok(SignedCertificate(
                    certificate::SignedCertificate::StakeDelegation(cert, auth),
                ))
            }
            2 => {
                let cert = certificate::OwnerStakeDelegation::read(buf)?;
                Ok(SignedCertificate(
                    certificate::SignedCertificate::OwnerStakeDelegation(cert, ()),
                ))
            }
            3 => {
                let cert = certificate::PoolRegistration::read(buf)?;
                let auth = Readable::read(buf)?;
                Ok(SignedCertificate(
                    certificate::SignedCertificate::PoolRegistration(cert, auth),
                ))
            }
            4 => {
                let cert = certificate::PoolRetirement::read(buf)?;
                let auth = Readable::read(buf)?;
                Ok(SignedCertificate(
                    certificate::SignedCertificate::PoolRetirement(cert, auth),
                ))
            }
            5 => {
                let cert = certificate::PoolUpdate::read(buf)?;
                let auth = Readable::read(buf)?;
                Ok(SignedCertificate(
                    certificate::SignedCertificate::PoolUpdate(cert, auth),
                ))
            }
            t => Err(ReadError::UnknownTag(t as u32))?,
        }
    }
}

custom_error! {pub CertificateToBech32Error
    Io { source: std::io::Error } = "Cannot serialize the Certificate",
    Bech32 { source: bech32::Error } = "Cannot create new Bech32",
}

custom_error! {pub CertificateFromBech32Error
    InvalidHRP { expected: String, actual: String } = "Invalid prefix, expected {expected} but read {actual}.",
    InvalidBase32 { source: bech32::Error } = "invalid base32",
    InvalidCertificate { source: chain_core::mempack::ReadError } = "Invalid certificate",
}

custom_error! {pub CertificateFromStrError
    InvalidCertificate { source: CertificateFromBech32Error } = "Invalid certificate",
    InvalidBech32 { source: bech32::Error } = "expected certificate in bech32",
}

impl Certificate {
    pub fn to_bech32(&self) -> Result<Bech32, CertificateToBech32Error> {
        use chain_core::property::Serialize as _;
        let bytes = self.serialize_as_vec()?;
        Ok(Bech32::new(CERTIFICATE_HRP.to_string(), bytes.to_base32())?)
    }

    pub fn from_bech32(bech32: &Bech32) -> Result<Self, CertificateFromBech32Error> {
        if bech32.hrp() != CERTIFICATE_HRP {
            return Err(CertificateFromBech32Error::InvalidHRP {
                expected: CERTIFICATE_HRP.to_owned(),
                actual: bech32.hrp().to_owned(),
            });
        }
        let bytes: Vec<u8> = Vec::from_base32(bech32.data())?;
        let mut buf = ReadBuf::from(&bytes);
        Certificate::read(&mut buf).map_err(CertificateFromBech32Error::from)
    }
}

impl SignedCertificate {
    pub fn to_bech32(&self) -> Result<Bech32, CertificateToBech32Error> {
        use chain_core::property::Serialize as _;
        let bytes = self.serialize_as_vec()?;
        Ok(Bech32::new(
            SIGNED_CERTIFICATE_HRP.to_string(),
            bytes.to_base32(),
        )?)
    }

    pub fn from_bech32(bech32: &Bech32) -> Result<Self, CertificateFromBech32Error> {
        if bech32.hrp() != SIGNED_CERTIFICATE_HRP {
            return Err(CertificateFromBech32Error::InvalidHRP {
                expected: SIGNED_CERTIFICATE_HRP.to_owned(),
                actual: bech32.hrp().to_owned(),
            });
        }
        let bytes: Vec<u8> = Vec::from_base32(bech32.data())?;
        let mut buf = ReadBuf::from(&bytes);
        SignedCertificate::read(&mut buf).map_err(CertificateFromBech32Error::from)
    }
}

/* ---------------- Display ------------------------------------------------ */

impl fmt::Display for Certificate {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_bech32().unwrap())
    }
}

impl FromStr for Certificate {
    type Err = CertificateFromStrError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bech32 = Bech32::from_str(s)?;
        Ok(Certificate::from_bech32(&bech32)?)
    }
}

impl fmt::Display for SignedCertificate {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_bech32().unwrap())
    }
}

impl FromStr for SignedCertificate {
    type Err = CertificateFromStrError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bech32 = Bech32::from_str(s)?;
        Ok(SignedCertificate::from_bech32(&bech32)?)
    }
}

/* ---------------- Conversion --------------------------------------------- */

impl From<certificate::Certificate> for Certificate {
    fn from(v: certificate::Certificate) -> Self {
        Certificate(v)
    }
}

impl From<Certificate> for certificate::Certificate {
    fn from(v: Certificate) -> Self {
        v.0
    }
}

impl From<certificate::SignedCertificate> for SignedCertificate {
    fn from(v: certificate::SignedCertificate) -> Self {
        SignedCertificate(v)
    }
}

impl From<SignedCertificate> for certificate::SignedCertificate {
    fn from(v: SignedCertificate) -> Self {
        v.0
    }
}

/* ------------------- Serde ----------------------------------------------- */

impl Serialize for Certificate {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::Error as _;

        let bech32 = self.to_bech32().map_err(S::Error::custom)?;

        bech32.to_string().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Certificate {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::Error as _;

        let bech32_str = String::deserialize(deserializer)?;
        let bech32: Bech32 = bech32_str.parse().map_err(D::Error::custom)?;

        Certificate::from_bech32(&bech32).map_err(D::Error::custom)
    }
}

impl Serialize for SignedCertificate {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::Error as _;

        let bech32 = self.to_bech32().map_err(S::Error::custom)?;

        bech32.to_string().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for SignedCertificate {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::Error as _;

        let bech32_str = String::deserialize(deserializer)?;
        let bech32: Bech32 = bech32_str.parse().map_err(D::Error::custom)?;

        SignedCertificate::from_bech32(&bech32).map_err(D::Error::custom)
    }
}
