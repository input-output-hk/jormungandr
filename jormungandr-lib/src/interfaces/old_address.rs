use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{fmt, str::FromStr};

/// OldAddress with the appropriate implementation for Serde API and
/// Display/FromStr interfaces.
///
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OldAddress(cardano_legacy_address::Addr);

/* ---------------- Display ------------------------------------------------ */

impl fmt::Display for OldAddress {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl FromStr for OldAddress {
    type Err = cardano_legacy_address::ParseExtendedAddrError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse().map(OldAddress)
    }
}

/* ---------------- AsRef -------------------------------------------------- */

impl AsRef<cardano_legacy_address::Addr> for OldAddress {
    fn as_ref(&self) -> &cardano_legacy_address::Addr {
        &self.0
    }
}
/* ---------------- Conversion --------------------------------------------- */

impl From<cardano_legacy_address::Addr> for OldAddress {
    fn from(v: cardano_legacy_address::Addr) -> Self {
        OldAddress(v)
    }
}

impl From<OldAddress> for cardano_legacy_address::Addr {
    fn from(v: OldAddress) -> Self {
        v.0
    }
}

/* ------------------- Serde ----------------------------------------------- */

impl Serialize for OldAddress {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0.to_string())
    }
}

impl<'de> Deserialize<'de> for OldAddress {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: String = String::deserialize(deserializer)?;
        cardano_legacy_address::Addr::from_str(&s)
            .map_err(|e| serde::de::Error::custom(e))
            .map(OldAddress)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use quickcheck::{Arbitrary, Gen, TestResult};

    impl Arbitrary for OldAddress {
        fn arbitrary<G>(g: &mut G) -> Self
        where
            G: Gen,
        {
            use ed25519_bip32::XPub;

            let mut bytes: [u8; 64] = [0; 64];
            for byte in bytes.iter_mut() {
                *byte = u8::arbitrary(g);
            }
            let xpub = XPub::from_bytes(bytes);

            let address =
                cardano_legacy_address::ExtendedAddr::new_simple(&xpub, Arbitrary::arbitrary(g));
            OldAddress(address.to_address())
        }
    }

    quickcheck! {
        fn address_display_parse(address: OldAddress) -> TestResult {
            let s = address.to_string();
            let address_dec: OldAddress = s.parse().unwrap();

            TestResult::from_bool(address == address_dec)
        }

        fn address_serde_human_readable_encode_decode(address: OldAddress) -> TestResult {
            let s = serde_yaml::to_string(&address).unwrap();
            let address_dec: OldAddress = serde_yaml::from_str(&s).unwrap();

            TestResult::from_bool(address == address_dec)
        }
    }
}
