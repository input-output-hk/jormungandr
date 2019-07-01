use chain_impl_mockchain::certificate;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Certificate(certificate::Certificate);

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
        use bech32::{Bech32, ToBase32 as _};
        use chain_core::property::Serialize as _;
        use serde::ser::Error as _;

        let bytes = self.0.serialize_as_vec().map_err(S::Error::custom)?;
        let bech32 =
            Bech32::new("cert".to_string(), bytes.to_base32()).map_err(S::Error::custom)?;

        format!("{}", bech32).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Certificate {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use bech32::{Bech32, FromBase32 as _};
        use chain_core::mempack::{ReadBuf, Readable as _};
        use serde::de::Error as _;

        let bech32_str = String::deserialize(deserializer)?;
        let bech32: Bech32 = bech32_str.parse().map_err(D::Error::custom)?;
        if bech32.hrp() != "cert" {
            return Err(D::Error::custom(format!(
                "Expecting certificate in bech32, with HRP 'cert'"
            )));
        }
        let bytes: Vec<u8> = Vec::from_base32(bech32.data()).map_err(D::Error::custom)?;
        let mut buf = ReadBuf::from(&bytes);
        certificate::Certificate::read(&mut buf)
            .map_err(D::Error::custom)
            .map(Certificate)
    }
}
