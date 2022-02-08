use bech32::{self, FromBase32 as _, ToBase32 as _};
use chain_core::{packer::Codec, property};
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
    fn serialize<W: std::io::Write>(
        &self,
        codec: &mut Codec<W>,
    ) -> Result<(), property::WriteError> {
        match &self.0 {
            certificate::Certificate::StakeDelegation(c) => {
                codec.put_bytes(&[1])?;
                codec.put_bytes(c.serialize().as_slice())?;
            }
            certificate::Certificate::OwnerStakeDelegation(c) => {
                codec.put_bytes(&[2])?;
                codec.put_bytes(c.serialize().as_slice())?;
            }
            certificate::Certificate::PoolRegistration(c) => {
                codec.put_bytes(&[3])?;
                codec.put_bytes(c.serialize().as_slice())?;
            }
            certificate::Certificate::PoolRetirement(c) => {
                codec.put_bytes(&[4])?;
                codec.put_bytes(c.serialize().as_slice())?;
            }
            certificate::Certificate::PoolUpdate(c) => {
                codec.put_bytes(&[5])?;
                codec.put_bytes(c.serialize().as_slice())?;
            }
            certificate::Certificate::VotePlan(c) => {
                codec.put_bytes(&[6])?;
                codec.put_bytes(c.serialize().as_slice())?;
            }
            certificate::Certificate::VoteCast(c) => {
                codec.put_bytes(&[7])?;
                codec.put_bytes(c.serialize().as_slice())?;
            }
            certificate::Certificate::VoteTally(c) => {
                codec.put_bytes(&[8])?;
                codec.put_bytes(c.serialize().as_slice())?;
            }
            certificate::Certificate::EncryptedVoteTally(c) => {
                codec.put_bytes(&[9])?;
                codec.put_bytes(c.serialize().as_slice())?;
            }
            certificate::Certificate::UpdateProposal(c) => {
                codec.put_bytes(&[10])?;
                codec.put_bytes(c.serialize().as_slice())?;
            }
            certificate::Certificate::UpdateVote(c) => {
                codec.put_bytes(&[11])?;
                codec.put_bytes(c.serialize().as_slice())?;
            }
            certificate::Certificate::MintToken(c) => {
                codec.put_bytes(&[12])?;
                codec.put_bytes(c.serialize().as_slice())?;
            }
        };
        Ok(())
    }
}

impl property::DeserializeFromSlice for Certificate {
    fn deserialize_from_slice(
        codec: &mut chain_core::packer::Codec<&[u8]>,
    ) -> Result<Self, property::ReadError> {
        match codec.get_u8()? {
            1 => {
                let cert = certificate::StakeDelegation::deserialize_from_slice(codec)?;
                Ok(Certificate(certificate::Certificate::StakeDelegation(cert)))
            }
            2 => {
                let cert = certificate::OwnerStakeDelegation::deserialize_from_slice(codec)?;
                Ok(Certificate(certificate::Certificate::OwnerStakeDelegation(
                    cert,
                )))
            }
            3 => {
                let cert = certificate::PoolRegistration::deserialize_from_slice(codec)?;
                Ok(Certificate(certificate::Certificate::PoolRegistration(
                    cert,
                )))
            }
            4 => {
                let cert = certificate::PoolRetirement::deserialize_from_slice(codec)?;
                Ok(Certificate(certificate::Certificate::PoolRetirement(cert)))
            }
            5 => {
                let cert = certificate::PoolUpdate::deserialize_from_slice(codec)?;
                Ok(Certificate(certificate::Certificate::PoolUpdate(cert)))
            }
            6 => {
                let cert = certificate::VotePlan::deserialize_from_slice(codec)?;
                Ok(Certificate(certificate::Certificate::VotePlan(cert)))
            }
            7 => {
                let cert = certificate::VoteCast::deserialize_from_slice(codec)?;
                Ok(Certificate(certificate::Certificate::VoteCast(cert)))
            }
            8 => {
                let cert = certificate::VoteTally::deserialize_from_slice(codec)?;
                Ok(Certificate(certificate::Certificate::VoteTally(cert)))
            }
            9 => {
                let cert = certificate::EncryptedVoteTally::deserialize_from_slice(codec)?;
                Ok(Certificate(certificate::Certificate::EncryptedVoteTally(
                    cert,
                )))
            }
            10 => {
                let cert = certificate::UpdateProposal::deserialize_from_slice(codec)?;
                Ok(Certificate(certificate::Certificate::UpdateProposal(cert)))
            }
            11 => {
                let cert = certificate::UpdateVote::deserialize_from_slice(codec)?;
                Ok(Certificate(certificate::Certificate::UpdateVote(cert)))
            }
            t => Err(property::ReadError::UnknownTag(t as u32)),
        }
    }
}

impl property::Serialize for SignedCertificate {
    fn serialize<W: std::io::Write>(
        &self,
        codec: &mut chain_core::packer::Codec<W>,
    ) -> Result<(), property::WriteError> {
        match &self.0 {
            certificate::SignedCertificate::StakeDelegation(c, a) => {
                codec.put_bytes(&[1])?;
                codec.put_bytes(c.serialize().as_slice())?;
                codec.put_bytes(a.serialize_in(ByteBuilder::new()).finalize().as_slice())?;
            }
            certificate::SignedCertificate::OwnerStakeDelegation(c, ()) => {
                codec.put_bytes(&[2])?;
                codec.put_bytes(c.serialize().as_slice())?;
            }
            certificate::SignedCertificate::PoolRegistration(c, a) => {
                codec.put_bytes(&[3])?;
                codec.put_bytes(c.serialize().as_slice())?;
                codec.put_bytes(a.serialize_in(ByteBuilder::new()).finalize().as_slice())?;
            }
            certificate::SignedCertificate::PoolRetirement(c, a) => {
                codec.put_bytes(&[4])?;
                codec.put_bytes(c.serialize().as_slice())?;
                codec.put_bytes(a.serialize_in(ByteBuilder::new()).finalize().as_slice())?;
            }
            certificate::SignedCertificate::PoolUpdate(c, a) => {
                codec.put_bytes(&[5])?;
                codec.put_bytes(c.serialize().as_slice())?;
                codec.put_bytes(a.serialize_in(ByteBuilder::new()).finalize().as_slice())?;
            }
            certificate::SignedCertificate::VotePlan(c, a) => {
                codec.put_bytes(&[6])?;
                codec.put_bytes(c.serialize().as_slice())?;
                codec.put_bytes(a.serialize_in(ByteBuilder::new()).finalize().as_slice())?;
            }
            certificate::SignedCertificate::VoteTally(c, a) => {
                codec.put_bytes(&[8])?;
                codec.put_bytes(c.serialize().as_slice())?;
                codec.put_bytes(a.serialize_in(ByteBuilder::new()).finalize().as_slice())?;
            }
            certificate::SignedCertificate::EncryptedVoteTally(c, a) => {
                codec.put_bytes(&[9])?;
                codec.put_bytes(c.serialize().as_slice())?;
                codec.put_bytes(a.serialize_in(ByteBuilder::new()).finalize().as_slice())?;
            }
            certificate::SignedCertificate::UpdateProposal(c, a) => {
                codec.put_bytes(&[10])?;
                codec.put_bytes(c.serialize().as_slice())?;
                codec.put_bytes(a.serialize_in(ByteBuilder::new()).finalize().as_slice())?;
            }
            certificate::SignedCertificate::UpdateVote(c, a) => {
                codec.put_bytes(&[11])?;
                codec.put_bytes(c.serialize().as_slice())?;
                codec.put_bytes(a.serialize_in(ByteBuilder::new()).finalize().as_slice())?;
            }
        };
        Ok(())
    }
}

impl property::DeserializeFromSlice for SignedCertificate {
    fn deserialize_from_slice(
        codec: &mut chain_core::packer::Codec<&[u8]>,
    ) -> Result<Self, property::ReadError> {
        match codec.get_u8()? {
            1 => {
                let cert = certificate::StakeDelegation::deserialize_from_slice(codec)?;
                let auth = property::DeserializeFromSlice::deserialize_from_slice(codec)?;
                Ok(SignedCertificate(
                    certificate::SignedCertificate::StakeDelegation(cert, auth),
                ))
            }
            2 => {
                let cert = certificate::OwnerStakeDelegation::deserialize_from_slice(codec)?;
                Ok(SignedCertificate(
                    certificate::SignedCertificate::OwnerStakeDelegation(cert, ()),
                ))
            }
            3 => {
                let cert = certificate::PoolRegistration::deserialize_from_slice(codec)?;
                let auth = property::DeserializeFromSlice::deserialize_from_slice(codec)?;
                Ok(SignedCertificate(
                    certificate::SignedCertificate::PoolRegistration(cert, auth),
                ))
            }
            4 => {
                let cert = certificate::PoolRetirement::deserialize_from_slice(codec)?;
                let auth = property::DeserializeFromSlice::deserialize_from_slice(codec)?;
                Ok(SignedCertificate(
                    certificate::SignedCertificate::PoolRetirement(cert, auth),
                ))
            }
            5 => {
                let cert = certificate::PoolUpdate::deserialize_from_slice(codec)?;
                let auth = property::DeserializeFromSlice::deserialize_from_slice(codec)?;
                Ok(SignedCertificate(
                    certificate::SignedCertificate::PoolUpdate(cert, auth),
                ))
            }
            6 => {
                let cert = certificate::VotePlan::deserialize_from_slice(codec)?;
                let auth = property::DeserializeFromSlice::deserialize_from_slice(codec)?;
                Ok(SignedCertificate(certificate::SignedCertificate::VotePlan(
                    cert, auth,
                )))
            }
            8 => {
                let cert = certificate::VoteTally::deserialize_from_slice(codec)?;
                let auth = property::DeserializeFromSlice::deserialize_from_slice(codec)?;
                Ok(SignedCertificate(
                    certificate::SignedCertificate::VoteTally(cert, auth),
                ))
            }
            9 => {
                let cert = certificate::EncryptedVoteTally::deserialize_from_slice(codec)?;
                let auth = property::DeserializeFromSlice::deserialize_from_slice(codec)?;
                Ok(SignedCertificate(
                    certificate::SignedCertificate::EncryptedVoteTally(cert, auth),
                ))
            }
            10 => {
                let cert = certificate::UpdateProposal::deserialize_from_slice(codec)?;
                let auth = property::DeserializeFromSlice::deserialize_from_slice(codec)?;
                Ok(SignedCertificate(
                    certificate::SignedCertificate::UpdateProposal(cert, auth),
                ))
            }
            11 => {
                let cert = certificate::UpdateVote::deserialize_from_slice(codec)?;
                let auth = property::DeserializeFromSlice::deserialize_from_slice(codec)?;
                Ok(SignedCertificate(
                    certificate::SignedCertificate::UpdateVote(cert, auth),
                ))
            }
            t => Err(property::ReadError::UnknownTag(t as u32)),
        }
    }
}

#[derive(Debug, Error)]
pub enum CertificateToBech32Error {
    #[error("Cannot serialize the Certificate")]
    Io(#[from] property::WriteError),
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
    InvalidCertificate(#[from] chain_core::property::ReadError),
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
        let bytes = property::Serialize::serialize_as_vec(&self)?;
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
        <Certificate as property::DeserializeFromSlice>::deserialize_from_slice(&mut Codec::new(
            bytes.as_slice(),
        ))
        .map_err(CertificateFromBech32Error::from)
    }
}

impl SignedCertificate {
    pub fn to_bech32m(&self) -> Result<String, CertificateToBech32Error> {
        let bytes = property::Serialize::serialize_as_vec(&self)?;
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
        <SignedCertificate as property::DeserializeFromSlice>::deserialize_from_slice(
            &mut Codec::new(bytes.as_slice()),
        )
        .map_err(CertificateFromBech32Error::from)
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
