//! Address creation and parsing
//!
//! Address components are:
//! * `HashedSpendingData` computed from `SpendingData`
//! * `Attributes`
//! * `AddrType`
//!
//! All this components form an `ExtendedAddr`, which serialized
//! to binary makes an `Addr`
//!

use base58;
use cbor;
use cbor_event::{self, de::Deserializer, se::Serializer};
use cryptoxide::blake2b::Blake2b;
use cryptoxide::digest::Digest;
use cryptoxide::sha3::Sha3;
use ed25519_bip32::XPub;

use std::{
    convert::TryFrom,
    fmt,
    io::{BufRead, Write},
};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
pub enum AddrType {
    ATPubKey,
}
impl fmt::Display for AddrType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AddrType::ATPubKey => write!(f, "Public Key"),
        }
    }
}
// [TkListLen 1, TkInt (fromEnum t)]
impl AddrType {
    fn from_u64(v: u64) -> Option<Self> {
        match v {
            0 => Some(AddrType::ATPubKey),
            _ => None,
        }
    }
    fn to_byte(self) -> u8 {
        match self {
            AddrType::ATPubKey => 0,
        }
    }
}
impl cbor_event::se::Serialize for AddrType {
    fn serialize<'se, W: Write>(
        &self,
        serializer: &'se mut Serializer<W>,
    ) -> cbor_event::Result<&'se mut Serializer<W>> {
        serializer.write_unsigned_integer(self.to_byte() as u64)
    }
}
impl cbor_event::de::Deserialize for AddrType {
    fn deserialize<R: BufRead>(reader: &mut Deserializer<R>) -> cbor_event::Result<Self> {
        match AddrType::from_u64(reader.unsigned_integer()?) {
            Some(addr_type) => Ok(addr_type),
            None => Err(cbor_event::Error::CustomError(format!("Invalid AddrType"))),
        }
    }
}

type HDAddressPayload = Vec<u8>;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct Attributes {
    pub derivation_path: Option<HDAddressPayload>,
    pub network_magic: Option<u32>,
}
impl Attributes {
    pub fn new_bootstrap_era(hdap: Option<HDAddressPayload>, network_magic: Option<u32>) -> Self {
        Attributes {
            derivation_path: hdap,
            network_magic,
        }
    }
}

const ATTRIBUTE_NAME_TAG_DERIVATION: u64 = 1;
const ATTRIBUTE_NAME_TAG_NETWORK_MAGIC: u64 = 2;

impl cbor_event::se::Serialize for Attributes {
    fn serialize<'se, W: Write>(
        &self,
        serializer: &'se mut Serializer<W>,
    ) -> cbor_event::Result<&'se mut Serializer<W>> {
        let mut len = 0;
        if let Some(_) = &self.derivation_path {
            len += 1
        };
        if let Some(_) = &self.network_magic {
            len += 1
        };
        let serializer = serializer.write_map(cbor_event::Len::Len(len))?;
        let serializer = match &self.derivation_path {
            &None => serializer,
            &Some(ref dp) => serializer
                .write_unsigned_integer(ATTRIBUTE_NAME_TAG_DERIVATION)?
                .write_bytes(&dp)?,
        };
        let serializer = match &self.network_magic {
            &None => serializer,
            &Some(network_magic) => serializer
                .write_unsigned_integer(ATTRIBUTE_NAME_TAG_NETWORK_MAGIC)?
                .write_bytes(cbor!(&network_magic)?)?,
        };
        Ok(serializer)
    }
}
impl cbor_event::de::Deserialize for Attributes {
    fn deserialize<R: BufRead>(reader: &mut Deserializer<R>) -> cbor_event::Result<Self> {
        let len = reader.map()?;
        let mut len = match len {
            cbor_event::Len::Indefinite => {
                return Err(cbor_event::Error::CustomError(format!(
                    "Invalid Attributes: received map of {:?} elements",
                    len
                )));
            }
            cbor_event::Len::Len(len) => len,
        };
        let mut derivation_path = None;
        let mut network_magic = None;
        while len > 0 {
            let key = reader.unsigned_integer()?;
            match key {
                ATTRIBUTE_NAME_TAG_DERIVATION => derivation_path = Some(reader.deserialize()?),
                ATTRIBUTE_NAME_TAG_NETWORK_MAGIC => {
                    // Yes, this is an integer encoded as CBOR encoded as Bytes in CBOR.
                    let bytes = reader.bytes()?;
                    let n = Deserializer::from(std::io::Cursor::new(bytes)).deserialize::<u32>()?;
                    network_magic = Some(n);
                }
                _ => {
                    return Err(cbor_event::Error::CustomError(format!(
                        "invalid Attribute key {}",
                        key
                    )));
                }
            }
            len -= 1;
        }
        Ok(Attributes {
            derivation_path,
            network_magic,
        })
    }
}

// calculate the hash of the data using SHA3 digest then using Blake2b224
fn sha3_then_blake2b224(data: &[u8]) -> [u8; 28] {
    let mut sh3 = Sha3::sha3_256();
    let mut sh3_out = [0; 32];
    sh3.input(data.as_ref());
    sh3.result(&mut sh3_out);

    let mut b2b = Blake2b::new(28);
    let mut out = [0; 28];
    b2b.input(&sh3_out[..]);
    b2b.result(&mut out);
    out
}

fn hash_spending_data(addr_type: AddrType, xpub: XPub, attrs: &Attributes) -> [u8; 28] {
    let buf = cbor!(&(&addr_type, &SpendingData(xpub), attrs))
        .expect("serialize the HashedSpendingData's digest data");
    sha3_then_blake2b224(&buf)
}

/// A valid cardano Address that is displayed in base58
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct Addr(Vec<u8>);

impl Addr {
    pub fn deconstruct(&self) -> ExtendedAddr {
        let mut raw = Deserializer::from(std::io::Cursor::new(&self.0));
        cbor_event::de::Deserialize::deserialize(&mut raw).unwrap() // unwrap should never fail from addr to extended addr
    }

    /// Check if the Addr can be reconstructed with a specific xpub
    pub fn identical_with_pubkey(&self, xpub: &XPub) -> bool {
        let ea = self.deconstruct();
        let newea = ExtendedAddr::new(xpub, ea.attributes);
        self == &newea.to_address()
    }

    /// mostly helper of the previous function, so not to have to expose the xpub construction
    pub fn identical_with_pubkey_raw(&self, xpub: &[u8]) -> bool {
        match XPub::from_slice(xpub) {
            Ok(xpub) => self.identical_with_pubkey(&xpub),
            _ => false,
        }
    }
}

impl AsRef<[u8]> for Addr {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl TryFrom<&[u8]> for Addr {
    type Error = cbor_event::Error;

    fn try_from(slice: &[u8]) -> Result<Self, Self::Error> {
        let mut v = Vec::new();
        // TODO we only want validation of slice here, but we don't have api to do that yet.
        {
            let mut raw = Deserializer::from(std::io::Cursor::new(&slice));
            let _: ExtendedAddr = cbor_event::de::Deserialize::deserialize(&mut raw)?;
        }
        v.extend_from_slice(slice);
        Ok(Addr(v))
    }
}

impl ::std::str::FromStr for Addr {
    type Err = ParseExtendedAddrError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes = base58::decode(s).map_err(ParseExtendedAddrError::Base58Error)?;

        Self::try_from(&bytes[..]).map_err(ParseExtendedAddrError::EncodingError)
    }
}

impl From<ExtendedAddr> for Addr {
    fn from(ea: ExtendedAddr) -> Self {
        ea.to_address()
    }
}

impl fmt::Display for Addr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", base58::encode(&self.0))
    }
}

impl cbor_event::se::Serialize for Addr {
    fn serialize<'se, W: Write>(
        &self,
        serializer: &'se mut Serializer<W>,
    ) -> cbor_event::Result<&'se mut Serializer<W>> {
        // Addr is already serialized
        serializer.write_raw_bytes(&self.0)
    }
}
impl cbor_event::de::Deserialize for Addr {
    fn deserialize<R: BufRead>(reader: &mut Deserializer<R>) -> cbor_event::Result<Self> {
        let ea: ExtendedAddr = cbor_event::de::Deserialize::deserialize(reader)?;
        Ok(ea.to_address())
    }
}

/// A valid cardano address deconstructed
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ExtendedAddr {
    pub addr: [u8; 28],
    pub attributes: Attributes,
}
impl ExtendedAddr {
    pub fn new(xpub: XPub, attrs: Attributes) -> Self {
        ExtendedAddr {
            addr: hash_spending_data(AddrType::ATPubKey, xpub, &attrs),
            attributes: attrs,
        }
    }

    // bootstrap era + no hdpayload address
    pub fn new_simple(xpub: XPub, network_magic: Option<u32>) -> Self {
        ExtendedAddr::new(xpub, Attributes::new_bootstrap_era(None, network_magic))
    }

    pub fn to_address(&self) -> Addr {
        Addr(cbor!(self).unwrap()) // unwrap should never fail from strongly typed extended addr to addr
    }
}
#[derive(Debug)]
pub enum ParseExtendedAddrError {
    EncodingError(cbor_event::Error),
    Base58Error(base58::Error),
}
impl ::std::str::FromStr for ExtendedAddr {
    type Err = ParseExtendedAddrError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes = base58::decode(s).map_err(ParseExtendedAddrError::Base58Error)?;

        Self::try_from(&bytes[..]).map_err(ParseExtendedAddrError::EncodingError)
    }
}
impl TryFrom<&[u8]> for ExtendedAddr {
    type Error = cbor_event::Error;

    fn try_from(slice: &[u8]) -> Result<Self, Self::Error> {
        let mut raw = Deserializer::from(std::io::Cursor::new(slice));
        cbor_event::de::Deserialize::deserialize(&mut raw)
    }
}
impl cbor_event::se::Serialize for ExtendedAddr {
    fn serialize<'se, W: Write>(
        &self,
        serializer: &'se mut Serializer<W>,
    ) -> cbor_event::Result<&'se mut Serializer<W>> {
        cbor::util::encode_with_crc32_(
            &(&self.addr, &self.attributes, &AddrType::ATPubKey),
            serializer,
        )?;
        Ok(serializer)
    }
}
impl cbor_event::de::Deserialize for ExtendedAddr {
    fn deserialize<R: BufRead>(reader: &mut Deserializer<R>) -> cbor_event::Result<Self> {
        let bytes = cbor::util::raw_with_crc32(reader)?;
        let mut raw = Deserializer::from(std::io::Cursor::new(bytes));
        raw.tuple(3, "ExtendedAddr")?;
        let addr = cbor_event::de::Deserialize::deserialize(&mut raw)?;
        let attributes = cbor_event::de::Deserialize::deserialize(&mut raw)?;
        let _addr_type: AddrType = cbor_event::de::Deserialize::deserialize(&mut raw)?;

        Ok(ExtendedAddr { addr, attributes })
    }
}
impl fmt::Display for ExtendedAddr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_address())
    }
}

const SPENDING_DATA_TAG_PUBKEY: u64 = 0;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct SpendingData(XPub);

impl cbor_event::se::Serialize for SpendingData {
    fn serialize<'se, W: Write>(
        &self,
        serializer: &'se mut Serializer<W>,
    ) -> cbor_event::Result<&'se mut Serializer<W>> {
        let ar: [u8; 64] = self.0.clone().into();
        serializer
            .write_array(cbor_event::Len::Len(2))?
            .write_unsigned_integer(SPENDING_DATA_TAG_PUBKEY)?
            .write_bytes(&ar[..])
    }
}

#[cfg(tests)]
mod test {
    pub fn it_works() {}
}
