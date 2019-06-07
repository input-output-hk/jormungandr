//! Address
//!
//! It uses a simple serialization format which is made to be concise:
//! * First byte contains the discrimination information (1 bit) and the kind of address (7 bits)
//! * Remaining bytes contains a kind specific encoding describe after.
//!
//! 3 kinds of address are currently supported:
//! * Single: Just a (spending) public key using the ED25519 algorithm
//! * Group: Same as single, but with a added (staking/group) public key
//!   using the ED25519 algorithm.
//! * Account: A account public key using the ED25519 algorithm
//!
//! Single key:
//!     DISCRIMINATION_BIT || SINGLE_KIND_TYPE (7 bits) || SPENDING_KEY
//!
//! Group key:
//!     DISCRIMINATION_BIT || GROUP_KIND_TYPE (7 bits) || SPENDING_KEY || ACCOUNT_KEY
//!
//! Account key:
//!     DISCRIMINATION_BIT || ACCOUNT_KIND_TYPE (7 bits) || ACCOUNT_KEY
//!
//! Multisig key:
//!     DISCRIMINATION_BIT || MULTISIG_KING_TYPE (7 bits) || MULTISIG_MERKLE_ROOT_PUBLIC_KEY
//!
//! Address human format is bech32 encoded
//!

#[cfg(test)]
#[macro_use]
extern crate quickcheck;

#[macro_use]
extern crate cfg_if;

use bech32::{Bech32, FromBase32, ToBase32};
use std::string::ToString;

use chain_crypto::{Ed25519, PublicKey, PublicKeyError};

use chain_core::mempack::{ReadBuf, ReadError, Readable};
use chain_core::property::{self, Serialize as PropertySerialize};

cfg_if! {
   if #[cfg(test)] {
        mod testing;
    } else if #[cfg(feature = "property-test-api")] {
        mod testing;
    }
}

// Allow to differentiate between address in
// production and testing setting, so that
// one type of address is not used in another setting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Discrimination {
    Production,
    Test,
}

/// Kind of an address, which include the possible variation of scheme
///
/// * Single address : just a single ed25519 spending public key
/// * Group address : an ed25519 spending public key followed by a group public key used for staking
/// * Account address : an ed25519 stake public key
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Kind {
    Single(PublicKey<Ed25519>),
    Group(PublicKey<Ed25519>, PublicKey<Ed25519>),
    Account(PublicKey<Ed25519>),
    Multisig([u8; 32]),
}

/// Kind Type of an address
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KindType {
    Single,
    Group,
    Account,
    Multisig,
}

/// Size of a Single address
pub const ADDR_SIZE_SINGLE: usize = 33;

/// Size of a Group address
pub const ADDR_SIZE_GROUP: usize = 65;

/// Size of an Account address
pub const ADDR_SIZE_ACCOUNT: usize = 33;

/// Size of an Multisig Account address
pub const ADDR_SIZE_MULTISIG: usize = 33;

const ADDR_KIND_LOW_SENTINEL: u8 = 0x2; /* anything under or equal to this is invalid */
pub const ADDR_KIND_SINGLE: u8 = 0x3;
pub const ADDR_KIND_GROUP: u8 = 0x4;
pub const ADDR_KIND_ACCOUNT: u8 = 0x5;
pub const ADDR_KIND_MULTISIG: u8 = 0x6;
const ADDR_KIND_SENTINEL: u8 = 0x7; /* anything above or equal to this is invalid */

impl KindType {
    pub fn to_value(&self) -> u8 {
        match self {
            KindType::Single => ADDR_KIND_SINGLE,
            KindType::Group => ADDR_KIND_GROUP,
            KindType::Account => ADDR_KIND_ACCOUNT,
            KindType::Multisig => ADDR_KIND_MULTISIG,
        }
    }
}

/// An unstructured address including the
/// discrimination and the kind of address
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Address(pub Discrimination, pub Kind);

impl Address {
    pub fn discrimination(&self) -> Discrimination {
        self.0
    }
    pub fn kind(&self) -> &Kind {
        &self.1
    }
}

#[derive(Debug)]
pub enum Error {
    EmptyAddress,
    InvalidKind,
    InvalidAddress,
    InvalidInternalEncoding,
    InvalidPrefix,
    MismatchPrefix,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::EmptyAddress => write!(f, "empty address"),
            Error::InvalidKind => write!(f, "invalid kind"),
            Error::InvalidAddress => write!(f, "invalid address"),
            Error::InvalidInternalEncoding => write!(f, "invalid internal encoding"),
            Error::InvalidPrefix => write!(f, "invalid prefix"),
            Error::MismatchPrefix => write!(f, "mismatch prefix"),
        }
    }
}
impl std::error::Error for Error {}

impl From<PublicKeyError> for Error {
    fn from(_: PublicKeyError) -> Error {
        Error::InvalidAddress
    }
}

impl From<bech32::Error> for Error {
    fn from(_: bech32::Error) -> Error {
        Error::InvalidInternalEncoding
    }
}

impl Address {
    /// Try to convert from_bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        // check the kind is valid and length
        is_valid_data(bytes)?;

        let discr = get_discrimination_value(bytes[0]);
        let kind = match get_kind_value(bytes[0]) {
            ADDR_KIND_SINGLE => {
                let spending = PublicKey::from_binary(&bytes[1..])?;
                Kind::Single(spending)
            }
            ADDR_KIND_GROUP => {
                let spending = PublicKey::from_binary(&bytes[1..33])?;
                let group = PublicKey::from_binary(&bytes[33..])?;

                Kind::Group(spending, group)
            }
            ADDR_KIND_ACCOUNT => {
                let stake_key = PublicKey::from_binary(&bytes[1..])?;
                Kind::Account(stake_key)
            }
            ADDR_KIND_MULTISIG => {
                let mut hash = [0u8; 32];
                hash.copy_from_slice(&bytes[1..33]);
                Kind::Multisig(hash)
            }
            _ => unreachable!(),
        };
        Ok(Address(discr, kind))
    }

    /// Return the size
    pub fn to_size(&self) -> usize {
        match self.1 {
            Kind::Single(_) => ADDR_SIZE_SINGLE,
            Kind::Group(_, _) => ADDR_SIZE_GROUP,
            Kind::Account(_) => ADDR_SIZE_ACCOUNT,
            Kind::Multisig(_) => ADDR_SIZE_MULTISIG,
        }
    }

    /// Return the Kind type of a given address
    fn to_kind_type(&self) -> KindType {
        match self.1 {
            Kind::Single(_) => KindType::Single,
            Kind::Group(_, _) => KindType::Group,
            Kind::Account(_) => KindType::Account,
            Kind::Multisig(_) => KindType::Multisig,
        }
    }

    fn to_kind_value(&self) -> u8 {
        self.to_kind_type().to_value()
    }

    /// Serialize an address into bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        self.serialize_as_vec()
            .expect("expect in memory allocation to always work")
    }

    /// create a base32 encoding of the byte serialization
    ///
    /// This is not the official normal human representation
    /// for the address, but is used for debug / other.
    pub fn base32(&self) -> String {
        let v = ToBase32::to_base32(&self.to_bytes());
        let alphabet = b"abcdefghijklmnopqrstuvwxyz234567";
        let mut out = Vec::new();
        for i in v {
            out.push(alphabet[i.to_u8() as usize])
        }
        unsafe { String::from_utf8_unchecked(out) }
    }

    pub fn public_key<'a>(&'a self) -> Option<&'a PublicKey<Ed25519>> {
        match self.1 {
            Kind::Single(ref pk) => Some(pk),
            Kind::Group(ref pk, _) => Some(pk),
            Kind::Account(ref pk) => Some(pk),
            Kind::Multisig(_) => None,
        }
    }
}

fn get_kind_value(first_byte: u8) -> u8 {
    first_byte & 0b0111_1111
}

fn get_discrimination_value(first_byte: u8) -> Discrimination {
    if (first_byte & 0b1000_0000) == 0 {
        Discrimination::Production
    } else {
        Discrimination::Test
    }
}

fn is_valid_data(bytes: &[u8]) -> Result<(Discrimination, KindType), Error> {
    if bytes.len() == 0 {
        return Err(Error::EmptyAddress);
    }
    let kind_type = get_kind_value(bytes[0]);
    if kind_type <= ADDR_KIND_LOW_SENTINEL || kind_type >= ADDR_KIND_SENTINEL {
        return Err(Error::InvalidKind);
    }
    let kty = match kind_type {
        ADDR_KIND_SINGLE => {
            if bytes.len() != ADDR_SIZE_SINGLE {
                return Err(Error::InvalidAddress);
            }
            KindType::Single
        }
        ADDR_KIND_GROUP => {
            if bytes.len() != ADDR_SIZE_GROUP {
                return Err(Error::InvalidAddress);
            }
            KindType::Group
        }
        ADDR_KIND_ACCOUNT => {
            if bytes.len() != ADDR_SIZE_ACCOUNT {
                return Err(Error::InvalidAddress);
            }
            KindType::Account
        }
        ADDR_KIND_MULTISIG => {
            if bytes.len() != ADDR_SIZE_MULTISIG {
                return Err(Error::InvalidAddress);
            }
            KindType::Multisig
        }
        _ => return Err(Error::InvalidKind),
    };
    Ok((get_discrimination_value(bytes[0]), kty))
}

/// A valid address in a human readable format
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddressReadable(String);

impl AddressReadable {
    const PRODUCTION_ADDRESS_PREFIX: &'static str = env!("PRODUCTION_ADDRESS_PREFIX");
    const TEST_ADDRESS_PREFIX: &'static str = env!("TEST_ADDRESS_PREFIX");

    pub fn as_string(&self) -> &str {
        &self.0
    }

    /// Validate from a String to create a valid AddressReadable
    pub fn from_string(s: &str) -> Result<Self, Error> {
        use std::str::FromStr;
        let r = Bech32::from_str(s)?;
        let expected_discrimination = if r.hrp() == Self::PRODUCTION_ADDRESS_PREFIX {
            Discrimination::Production
        } else if r.hrp() == Self::TEST_ADDRESS_PREFIX {
            Discrimination::Test
        } else {
            return Err(Error::InvalidPrefix);
        };
        let dat = Vec::from_base32(r.data())?;
        let (discrimination, _) = is_valid_data(&dat[..])?;
        if discrimination != expected_discrimination {
            return Err(Error::MismatchPrefix);
        }
        Ok(AddressReadable(s.to_string()))
    }

    /// Create a new AddressReadable from an encoded address
    pub fn from_address(addr: &Address) -> Self {
        let v = ToBase32::to_base32(&addr.to_bytes());
        let prefix = match addr.0 {
            Discrimination::Production => Self::PRODUCTION_ADDRESS_PREFIX.to_string(),
            Discrimination::Test => Self::TEST_ADDRESS_PREFIX.to_string(),
        };
        let r = Bech32::new(prefix, v).unwrap();
        AddressReadable(r.to_string())
    }

    /// Convert a valid AddressReadable to an decoded address
    pub fn to_address(&self) -> Address {
        use std::str::FromStr;
        // the data has been verified ahead of time, so all unwrap are safe
        let r = Bech32::from_str(&self.0).unwrap();
        let dat = Vec::from_base32(r.data()).unwrap();
        Address::from_bytes(&dat[..]).unwrap()
    }
}

impl std::fmt::Display for AddressReadable {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for AddressReadable {
    type Err = Error;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        AddressReadable::from_string(s)
    }
}

impl PropertySerialize for Address {
    type Error = std::io::Error;

    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        use chain_core::packer::*;
        use std::io::Write;
        let mut codec = Codec::new(writer);

        let first_byte = match self.0 {
            Discrimination::Production => self.to_kind_value(),
            Discrimination::Test => self.to_kind_value() | 0b1000_0000,
        };
        codec.put_u8(first_byte)?;
        match &self.1 {
            Kind::Single(spend) => codec.write_all(spend.as_ref())?,
            Kind::Group(spend, group) => {
                codec.write_all(spend.as_ref())?;
                codec.write_all(group.as_ref())?;
            }
            Kind::Account(stake_key) => codec.write_all(stake_key.as_ref())?,
            Kind::Multisig(hash) => codec.write_all(&hash[..])?,
        };

        Ok(())
    }

    fn serialize_as_vec(&self) -> Result<Vec<u8>, Self::Error> {
        let mut data = Vec::with_capacity(self.to_size());
        self.serialize(&mut data)?;
        Ok(data)
    }
}
impl property::Deserialize for Address {
    type Error = std::io::Error;

    fn deserialize<R: std::io::BufRead>(reader: R) -> Result<Self, Self::Error> {
        use chain_core::packer::*;
        use std::io::Read;
        let mut codec = Codec::new(reader);
        // is_valid_data(bytes)?;

        let byte = codec.get_u8()?;

        let discr = get_discrimination_value(byte);
        let kind = match get_kind_value(byte) {
            ADDR_KIND_SINGLE => {
                let mut bytes = [0u8; 32];
                codec.read_exact(&mut bytes)?;
                let spending = PublicKey::from_binary(&bytes[..]).map_err(|err| {
                    std::io::Error::new(std::io::ErrorKind::InvalidData, Box::new(err))
                })?;
                Kind::Single(spending)
            }
            ADDR_KIND_GROUP => {
                let mut bytes = [0u8; 32];
                codec.read_exact(&mut bytes)?;
                let spending = PublicKey::from_binary(&bytes[..]).map_err(|err| {
                    std::io::Error::new(std::io::ErrorKind::InvalidData, Box::new(err))
                })?;
                let mut bytes = [0u8; 32];
                codec.read_exact(&mut bytes)?;
                let group = PublicKey::from_binary(&bytes[..]).map_err(|err| {
                    std::io::Error::new(std::io::ErrorKind::InvalidData, Box::new(err))
                })?;
                Kind::Group(spending, group)
            }
            ADDR_KIND_ACCOUNT => {
                let mut bytes = [0u8; 32];
                codec.read_exact(&mut bytes)?;
                let stake_key = PublicKey::from_binary(&bytes[..]).map_err(|err| {
                    std::io::Error::new(std::io::ErrorKind::InvalidData, Box::new(err))
                })?;
                Kind::Account(stake_key)
            }
            ADDR_KIND_MULTISIG => {
                let mut bytes = [0u8; 32];
                codec.read_exact(&mut bytes)?;
                Kind::Multisig(bytes)
            }
            _ => unreachable!(),
        };
        Ok(Address(discr, kind))
    }
}

fn chain_crypto_err(e: chain_crypto::PublicKeyError) -> ReadError {
    match e {
        PublicKeyError::SizeInvalid => {
            ReadError::StructureInvalid("publickey size invalid".to_string())
        }
        PublicKeyError::StructureInvalid => {
            ReadError::StructureInvalid("publickey structure invalid".to_string())
        }
    }
}

impl Readable for Address {
    fn read<'a>(buf: &mut ReadBuf<'a>) -> Result<Self, ReadError> {
        let byte = buf.get_u8()?;
        let discr = get_discrimination_value(byte);
        let kind = match get_kind_value(byte) {
            ADDR_KIND_SINGLE => {
                let bytes = <[u8; 32]>::read(buf)?;
                let spending = PublicKey::from_binary(&bytes[..]).map_err(chain_crypto_err)?;
                Kind::Single(spending)
            }
            ADDR_KIND_GROUP => {
                let bytes = <[u8; 32]>::read(buf)?;
                let spending = PublicKey::from_binary(&bytes[..]).map_err(chain_crypto_err)?;
                let bytes = <[u8; 32]>::read(buf)?;
                let group = PublicKey::from_binary(&bytes[..]).map_err(chain_crypto_err)?;
                Kind::Group(spending, group)
            }
            ADDR_KIND_ACCOUNT => {
                let bytes = <[u8; 32]>::read(buf)?;
                let stake_key = PublicKey::from_binary(&bytes[..]).map_err(chain_crypto_err)?;
                Kind::Account(stake_key)
            }
            ADDR_KIND_MULTISIG => {
                let bytes = <[u8; 32]>::read(buf)?;
                Kind::Multisig(bytes)
            }
            n => return Err(ReadError::UnknownTag(n as u32)),
        };
        Ok(Address(discr, kind))
    }
}

/// error that can happen when parsing the Discrimination
/// from a string
#[derive(Debug)]
pub struct ParseDiscriminationError(String);
impl std::fmt::Display for ParseDiscriminationError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "Invalid Address Discrimination `{}'. Expected `production' or `test'.",
            self.0
        )
    }
}
impl std::error::Error for ParseDiscriminationError {}

impl std::fmt::Display for Discrimination {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Discrimination::Production => write!(f, "production"),
            Discrimination::Test => write!(f, "test"),
        }
    }
}
impl std::str::FromStr for Discrimination {
    type Err = ParseDiscriminationError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "production" => Ok(Discrimination::Production),
            "test" => Ok(Discrimination::Test),
            _ => Err(ParseDiscriminationError(s.to_owned())),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn property_serialize_deserialize(addr: &Address) {
        let data = addr.to_bytes();
        let r = Address::from_bytes(&data[..]).unwrap();
        assert_eq!(&r, addr);
    }

    fn expected_base32(addr: &Address, expected: &'static str) {
        assert_eq!(addr.base32(), expected.to_string());
    }

    fn expected_bech32(addr: &Address, expected: &'static str) {
        assert_eq!(
            AddressReadable::from_address(&addr),
            AddressReadable(expected.to_string())
        );
    }

    fn property_readable(addr: &Address) {
        let ar = AddressReadable::from_address(addr);
        let a = ar.to_address();
        let ar2 =
            AddressReadable::from_string(ar.as_string()).expect("address is readable from string");
        assert_eq!(addr, &a);
        assert_eq!(ar, ar2);
    }

    quickcheck! {
        fn from_address_to_address(address: Address) -> bool {
            let readable = AddressReadable::from_address(&address);
            let decoded  = readable.to_address();

             address == decoded
        }

         fn to_bytes_from_bytes(address: Address) -> bool {
            let readable = address.to_bytes();
            let decoded  = Address::from_bytes(&readable).unwrap();

             address == decoded
        }
    }

    #[test]
    fn unit_tests() {
        let fake_spendingkey: PublicKey<Ed25519> = PublicKey::from_binary(&[
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
            25, 26, 27, 28, 29, 30, 31, 32,
        ])
        .unwrap();
        let fake_groupkey: PublicKey<Ed25519> = PublicKey::from_binary(&[
            41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62,
            63, 64, 65, 66, 67, 68, 69, 70, 71, 72,
        ])
        .unwrap();
        let fake_accountkey: PublicKey<Ed25519> = PublicKey::from_binary(&[
            41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62,
            63, 64, 65, 66, 67, 68, 69, 70, 71, 72,
        ])
        .unwrap();

        {
            let addr = Address(
                Discrimination::Production,
                Kind::Single(fake_spendingkey.clone()),
            );
            property_serialize_deserialize(&addr);
            property_readable(&addr);
            expected_base32(
                &addr,
                "amaqeayeaudaocajbifqydiob4ibceqtcqkrmfyydenbwha5dypsa",
            );
            expected_bech32(
                &addr,
                "ca1qvqsyqcyq5rqwzqfpg9scrgwpugpzysnzs23v9ccrydpk8qarc0jqxuzx4s",
            );
        }

        {
            let addr = Address(
                Discrimination::Production,
                Kind::Group(fake_spendingkey.clone(), fake_groupkey.clone()),
            );
            property_serialize_deserialize(&addr);
            property_readable(&addr);
            expected_bech32(&addr, "ca1qsqsyqcyq5rqwzqfpg9scrgwpugpzysnzs23v9ccrydpk8qarc0jq2f29vkz6t30xqcnyve5x5mrwwpe8ganc0f78aqyzsjrg3z5v36gguhxny");
            expected_base32(&addr, "aqaqeayeaudaocajbifqydiob4ibceqtcqkrmfyydenbwha5dypsakjkfmwc2lrpgaytemzugu3doobzhi5typj6h5aecqsdircumr2i");
        }

        {
            let addr = Address(
                Discrimination::Test,
                Kind::Group(fake_spendingkey.clone(), fake_groupkey.clone()),
            );
            property_serialize_deserialize(&addr);
            property_readable(&addr);
            expected_bech32(&addr, "ta1ssqsyqcyq5rqwzqfpg9scrgwpugpzysnzs23v9ccrydpk8qarc0jq2f29vkz6t30xqcnyve5x5mrwwpe8ganc0f78aqyzsjrg3z5v36ge5qsky");
            expected_base32(&addr, "qqaqeayeaudaocajbifqydiob4ibceqtcqkrmfyydenbwha5dypsakjkfmwc2lrpgaytemzugu3doobzhi5typj6h5aecqsdircumr2i");
        }

        {
            let addr = Address(Discrimination::Test, Kind::Account(fake_accountkey));
            property_serialize_deserialize(&addr);
            property_readable(&addr);
            expected_base32(
                &addr,
                "quusukzmfuxc6mbrgiztinjwg44dsor3hq6t4p2aifbegrcfizduq",
            );
            expected_bech32(
                &addr,
                "ta1s55j52ev95hz7vp3xgengdfkxuurjw3m8s7nu06qg9pyx3z9ger5s28ezm6",
            );
        }
    }
}
