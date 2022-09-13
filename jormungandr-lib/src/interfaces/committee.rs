pub use chain_impl_mockchain::vote::CommitteeId;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// remove serde encoding for the CommitteeId
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(remote = "CommitteeId")]
pub struct CommitteeIdDef(#[serde(getter = "get_bytes")] [u8; CommitteeId::COMMITTEE_ID_SIZE]);

impl From<CommitteeIdDef> for CommitteeId {
    fn from(committee_id_def: CommitteeIdDef) -> Self {
        Self::from(committee_id_def.0)
    }
}

impl From<CommitteeId> for CommitteeIdDef {
    fn from(committee_id: CommitteeId) -> Self {
        Self(get_bytes(&committee_id))
    }
}

impl From<[u8; CommitteeId::COMMITTEE_ID_SIZE]> for CommitteeIdDef {
    fn from(committee_id: [u8; CommitteeId::COMMITTEE_ID_SIZE]) -> Self {
        Self(committee_id)
    }
}

impl CommitteeIdDef {
    /// returns the identifier encoded in hexadecimal string
    pub fn to_hex(self) -> String {
        hex::encode(self.0)
    }

    /// read the identifier from the hexadecimal string
    pub fn from_hex(s: &str) -> Result<Self, hex::FromHexError> {
        CommitteeId::from_hex(s).map(Into::into)
    }
}

fn get_bytes(committee_id: &CommitteeId) -> [u8; CommitteeId::COMMITTEE_ID_SIZE] {
    let mut bytes = [0; CommitteeId::COMMITTEE_ID_SIZE];
    bytes.copy_from_slice(committee_id.as_ref());
    bytes
}

/* ------------------- Serde ----------------------------------------------- */

impl Serialize for CommitteeIdDef {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if serializer.is_human_readable() {
            let hex = hex::encode(self.0);
            serializer.serialize_str(&hex)
        } else {
            serializer.serialize_bytes(&self.0)
        }
    }
}

impl<'de> Deserialize<'de> for CommitteeIdDef {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let bytes = if deserializer.is_human_readable() {
            let s: String = String::deserialize(deserializer)?;
            let mut bytes = [0; CommitteeId::COMMITTEE_ID_SIZE];
            hex::decode_to_slice(&s, &mut bytes).map_err(serde::de::Error::custom)?;
            bytes
        } else {
            let b: Vec<u8> = Vec::deserialize(deserializer)?;
            if b.len() != CommitteeId::COMMITTEE_ID_SIZE {
                return Err(serde::de::Error::custom("not enough bytes"));
            }

            let mut bytes = [0; CommitteeId::COMMITTEE_ID_SIZE];
            bytes.copy_from_slice(&b);
            bytes
        };

        Ok(Self(bytes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quickcheck::{Arbitrary, Gen};

    impl Arbitrary for CommitteeIdDef {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            let mut cid = CommitteeIdDef([0; CommitteeId::COMMITTEE_ID_SIZE]);
            g.fill_bytes(&mut cid.0);
            cid
        }
    }
}
