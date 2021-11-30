use bech32::{self, FromBase32 as _, ToBase32 as _};
use chain_core::mempack::{ReadBuf, ReadError, Readable};
use chain_core::property;
use chain_impl_mockchain::certificate;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{fmt, str::FromStr};
use thiserror::Error;
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
            certificate::SignedCertificate::VotePlan(c, _) => {
                Certificate(certificate::Certificate::VotePlan(c))
            }
            certificate::SignedCertificate::VoteTally(c, _) => {
                Certificate(certificate::Certificate::VoteTally(c))
            }
            certificate::SignedCertificate::EncryptedVoteTally(c, _) => {
                Certificate(certificate::Certificate::EncryptedVoteTally(c))
            }
            certificate::SignedCertificate::UpdateProposal(c, _) => {
                Certificate(certificate::Certificate::UpdateProposal(c))
            }
            certificate::SignedCertificate::UpdateVote(c, _) => {
                Certificate(certificate::Certificate::UpdateVote(c))
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
                writer.write_all(c.serialize().as_slice())?;
            }
            certificate::Certificate::OwnerStakeDelegation(c) => {
                writer.write_all(&[2])?;
                writer.write_all(c.serialize().as_slice())?;
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
            certificate::Certificate::VotePlan(c) => {
                writer.write_all(&[6])?;
                writer.write_all(c.serialize().as_slice())?;
            }
            certificate::Certificate::VoteCast(c) => {
                writer.write_all(&[7])?;
                writer.write_all(c.serialize().as_slice())?;
            }
            certificate::Certificate::VoteTally(c) => {
                writer.write_all(&[8])?;
                writer.write_all(c.serialize().as_slice())?;
            }
            certificate::Certificate::EncryptedVoteTally(c) => {
                writer.write_all(&[9])?;
                writer.write_all(c.serialize().as_slice())?;
            }
            certificate::Certificate::UpdateProposal(c) => {
                writer.write_all(&[10])?;
                writer.write_all(c.serialize().as_slice())?;
            }
            certificate::Certificate::UpdateVote(c) => {
                writer.write_all(&[11])?;
                writer.write_all(c.serialize().as_slice())?;
            }
            certificate::Certificate::MintToken(c) => {
                writer.write_all(&[12])?;
                writer.write_all(c.serialize().as_slice())?;
            }
        };
        Ok(())
    }
}

impl Readable for Certificate {
    fn read<'a>(buf: &mut ReadBuf<'_>) -> Result<Self, ReadError> {
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
            6 => {
                let cert = certificate::VotePlan::read(buf)?;
                Ok(Certificate(certificate::Certificate::VotePlan(cert)))
            }
            7 => {
                let cert = certificate::VoteCast::read(buf)?;
                Ok(Certificate(certificate::Certificate::VoteCast(cert)))
            }
            8 => {
                let cert = certificate::VoteTally::read(buf)?;
                Ok(Certificate(certificate::Certificate::VoteTally(cert)))
            }
            9 => {
                let cert = certificate::EncryptedVoteTally::read(buf)?;
                Ok(Certificate(certificate::Certificate::EncryptedVoteTally(
                    cert,
                )))
            }
            10 => {
                let cert = certificate::UpdateProposal::read(buf)?;
                Ok(Certificate(certificate::Certificate::UpdateProposal(cert)))
            }
            11 => {
                let cert = certificate::UpdateVote::read(buf)?;
                Ok(Certificate(certificate::Certificate::UpdateVote(cert)))
            }
            t => Err(ReadError::UnknownTag(t as u32)),
        }
    }
}

impl property::Serialize for SignedCertificate {
    type Error = std::io::Error;
    fn serialize<W: std::io::Write>(&self, mut writer: W) -> Result<(), Self::Error> {
        match &self.0 {
            certificate::SignedCertificate::StakeDelegation(c, a) => {
                writer.write_all(&[1])?;
                writer.write_all(c.serialize().as_slice())?;
                writer.write_all(a.serialize_in(ByteBuilder::new()).finalize().as_slice())?;
            }
            certificate::SignedCertificate::OwnerStakeDelegation(c, ()) => {
                writer.write_all(&[2])?;
                writer.write_all(c.serialize().as_slice())?;
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
            certificate::SignedCertificate::VotePlan(c, a) => {
                writer.write_all(&[6])?;
                writer.write_all(c.serialize().as_slice())?;
                writer.write_all(a.serialize_in(ByteBuilder::new()).finalize().as_slice())?;
            }
            certificate::SignedCertificate::VoteTally(c, a) => {
                writer.write_all(&[8])?;
                writer.write_all(c.serialize().as_slice())?;
                writer.write_all(a.serialize_in(ByteBuilder::new()).finalize().as_slice())?;
            }
            certificate::SignedCertificate::EncryptedVoteTally(c, a) => {
                writer.write_all(&[9])?;
                writer.write_all(c.serialize().as_slice())?;
                writer.write_all(a.serialize_in(ByteBuilder::new()).finalize().as_slice())?;
            }
            certificate::SignedCertificate::UpdateProposal(c, a) => {
                writer.write_all(&[10])?;
                writer.write_all(c.serialize().as_slice())?;
                writer.write_all(a.serialize_in(ByteBuilder::new()).finalize().as_slice())?;
            }
            certificate::SignedCertificate::UpdateVote(c, a) => {
                writer.write_all(&[11])?;
                writer.write_all(c.serialize().as_slice())?;
                writer.write_all(a.serialize_in(ByteBuilder::new()).finalize().as_slice())?;
            }
        };
        Ok(())
    }
}

impl Readable for SignedCertificate {
    fn read<'a>(buf: &mut ReadBuf<'_>) -> Result<Self, ReadError> {
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
            6 => {
                let cert = certificate::VotePlan::read(buf)?;
                let auth = Readable::read(buf)?;
                Ok(SignedCertificate(certificate::SignedCertificate::VotePlan(
                    cert, auth,
                )))
            }
            8 => {
                let cert = certificate::VoteTally::read(buf)?;
                let auth = Readable::read(buf)?;
                Ok(SignedCertificate(
                    certificate::SignedCertificate::VoteTally(cert, auth),
                ))
            }
            9 => {
                let cert = certificate::EncryptedVoteTally::read(buf)?;
                let auth = Readable::read(buf)?;
                Ok(SignedCertificate(
                    certificate::SignedCertificate::EncryptedVoteTally(cert, auth),
                ))
            }
            10 => {
                let cert = certificate::UpdateProposal::read(buf)?;
                let auth = Readable::read(buf)?;
                Ok(SignedCertificate(
                    certificate::SignedCertificate::UpdateProposal(cert, auth),
                ))
            }
            11 => {
                let cert = certificate::UpdateVote::read(buf)?;
                let auth = Readable::read(buf)?;
                Ok(SignedCertificate(
                    certificate::SignedCertificate::UpdateVote(cert, auth),
                ))
            }
            t => Err(ReadError::UnknownTag(t as u32)),
        }
    }
}

#[derive(Debug, Error)]
pub enum CertificateToBech32Error {
    #[error("Cannot serialize the Certificate")]
    Io(#[from] std::io::Error),
    #[error("Cannot create new Bech32")]
    Bech32(#[from] bech32::Error),
}

#[derive(Debug, Error)]
pub enum CertificateFromBech32Error {
    #[error("Invalid prefix, expected {expected} but read {actual}")]
    InvalidHrp { expected: String, actual: String },
    #[error("invalid base32")]
    InvalidBase32(#[from] bech32::Error),
    #[error("Invalid certificate")]
    InvalidCertificate(#[from] chain_core::mempack::ReadError),
}

#[derive(Debug, Error)]
pub enum CertificateFromStrError {
    #[error("Invalid certificate")]
    InvalidCertificate(#[from] CertificateFromBech32Error),
    #[error("expected certificate in bech32")]
    InvalidBech32(#[from] bech32::Error),
}

/// Use bech32m variant to serialize a certificate as its length it's not fixed
/// but allow to read original bech32 formatted certificates for backward compatibility
impl Certificate {
    pub fn to_bech32m(&self) -> Result<String, CertificateToBech32Error> {
        use chain_core::property::Serialize as _;
        let bytes = self.serialize_as_vec()?;
        // jormungandr_lib::Certificate is only used in jcli so we don't
        Ok(bech32::encode(
            CERTIFICATE_HRP,
            &bytes.to_base32(),
            bech32::Variant::Bech32m,
        )?)
    }

    pub fn from_bech32(bech32: &str) -> Result<Self, CertificateFromBech32Error> {
        let (hrp, data, _variant) = bech32::decode(bech32)?;
        if hrp != CERTIFICATE_HRP {
            return Err(CertificateFromBech32Error::InvalidHrp {
                expected: CERTIFICATE_HRP.to_owned(),
                actual: hrp,
            });
        }
        let bytes: Vec<u8> = Vec::from_base32(&data)?;
        let mut buf = ReadBuf::from(&bytes);
        Certificate::read(&mut buf).map_err(CertificateFromBech32Error::from)
    }
}

impl SignedCertificate {
    pub fn to_bech32m(&self) -> Result<String, CertificateToBech32Error> {
        use chain_core::property::Serialize as _;
        let bytes = self.serialize_as_vec()?;
        Ok(bech32::encode(
            SIGNED_CERTIFICATE_HRP,
            &bytes.to_base32(),
            bech32::Variant::Bech32m,
        )?)
    }

    pub fn from_bech32(bech32: &str) -> Result<Self, CertificateFromBech32Error> {
        let (hrp, data, _variant) = bech32::decode(bech32)?;
        if hrp != SIGNED_CERTIFICATE_HRP {
            return Err(CertificateFromBech32Error::InvalidHrp {
                expected: SIGNED_CERTIFICATE_HRP.to_owned(),
                actual: hrp,
            });
        }
        let bytes: Vec<u8> = Vec::from_base32(&data)?;
        let mut buf = ReadBuf::from(&bytes);
        SignedCertificate::read(&mut buf).map_err(CertificateFromBech32Error::from)
    }
}

/* ---------------- Display ------------------------------------------------ */

impl fmt::Display for Certificate {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_bech32m().unwrap())
    }
}

impl FromStr for Certificate {
    type Err = CertificateFromStrError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Certificate::from_bech32(s)?)
    }
}

impl fmt::Display for SignedCertificate {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_bech32m().unwrap())
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

        let bech32 = self.to_bech32m().map_err(S::Error::custom)?;

        bech32.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Certificate {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::Error as _;

        let bech32_str = String::deserialize(deserializer)?;
        Certificate::from_bech32(&bech32_str).map_err(D::Error::custom)
    }
}

impl Serialize for SignedCertificate {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::Error as _;

        let bech32 = self.to_bech32m().map_err(S::Error::custom)?;
        bech32.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for SignedCertificate {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::Error as _;

        let bech32_str = String::deserialize(deserializer)?;
        SignedCertificate::from_bech32(&bech32_str).map_err(D::Error::custom)
    }
}
