use bech32::{Bech32, FromBase32, ToBase32};
use chain_core::mempack::{ReadBuf, ReadError, Readable};
use chain_core::property::Serialize;
use chain_impl_mockchain::certificate::Certificate;

type StaticStr = &'static str;

custom_error! {pub Error
    Bech32 { source: bech32::Error } = "failed to parse bech32",
    Format { source: ReadError } = "Invalid format",
    Hrp { expected: StaticStr, actual: String } = "Invalid bech32 prefix, expected: `{expected}'",
}

pub fn serialize_to_bech32(cert: &Certificate) -> Result<Bech32, Error> {
    let bytes = cert.serialize_as_vec().unwrap();
    Bech32::new("cert".to_string(), bytes.to_base32()).map_err(Into::into)
}

pub fn deserialize_from_bech32(bech32_str: &str) -> Result<Certificate, Error> {
    let bech32: Bech32 = bech32_str.parse()?;
    if bech32.hrp() != "cert" {
        return Err(Error::Hrp {
            expected: "cert",
            actual: bech32.hrp().to_string(),
        });
    }
    let bytes = Vec::<u8>::from_base32(bech32.data())?;
    let mut buf = ReadBuf::from(&bytes);
    Certificate::read(&mut buf).map_err(Into::into)
}
