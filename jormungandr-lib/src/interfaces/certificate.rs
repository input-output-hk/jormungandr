use bech32::{Bech32, FromBase32 as _, ToBase32 as _};
use chain_impl_mockchain::certificate;
use chain_core::property;
use chain_core::mempack::{Readable, ReadBuf, ReadError};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{fmt, str::FromStr};

#[derive(Clone, Debug)]
pub struct Certificate(pub certificate::Certificate);

impl PartialEq for Certificate {
    fn eq(&self, other: &Self) -> bool {
        use property::Serialize as _;
        match (self.serialize_as_vec(),other.serialize_as_vec()) {
            (Ok(a), Ok(b)) => a == b,
            _ => false,
        }
    }
}

impl property::Serialize for Certificate {
    type Error = std::io::Error;
    fn serialize<W: std::io::Write>(&self, mut writer: W) -> Result<(), Self::Error> {
        match &self.0 {
            certificate::Certificate::StakeDelegation(c) => { writer.write_all(&[1])?; c.serialize(&mut writer)? },
            certificate::Certificate::OwnerStakeDelegation(c) => { writer.write_all(&[2])?; c.serialize(&mut writer)? },
            certificate::Certificate::PoolRegistration(c) => { writer.write_all(&[3])?; writer.write_all(c.serialize().as_slice())? },
            certificate::Certificate::PoolManagement(c) => { writer.write_all(&[4])?; writer.write_all(c.serialize().as_slice())? },
        };
        Ok(())
    }
}

impl Readable for Certificate {
    fn read<'a>(buf: &mut ReadBuf<'a>) -> Result<Self, ReadError> {
        match buf.get_u8()? {
            1 => Ok(Certificate(certificate::Certificate::StakeDelegation(certificate::StakeDelegation::read(buf)?))),
            2 => Ok(Certificate(certificate::Certificate::OwnerStakeDelegation(certificate::OwnerStakeDelegation::read(buf)?))),
            3 => Ok(Certificate(certificate::Certificate::PoolRegistration(certificate::PoolRegistration::read(buf)?))),
            4 => Ok(Certificate(certificate::Certificate::PoolManagement(certificate::PoolManagement::read(buf)?))),
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
        Ok(Bech32::new("cert".to_string(), bytes.to_base32())?)
    }

    pub fn from_bech32(bech32: &Bech32) -> Result<Self, CertificateFromBech32Error> {
        if bech32.hrp() != "cert" {
            return Err(CertificateFromBech32Error::InvalidHRP {
                expected: "cert".to_owned(),
                actual: bech32.hrp().to_owned(),
            });
        }
        let bytes: Vec<u8> = Vec::from_base32(bech32.data())?;
        let mut buf = ReadBuf::from(&bytes);
        Certificate::read(&mut buf)
            .map_err(CertificateFromBech32Error::from)
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
