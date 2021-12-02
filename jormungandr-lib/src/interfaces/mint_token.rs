use super::Value;
use crate::crypto::account::Identifier;
use chain_impl_mockchain::{
    certificate,
    tokens::{minting_policy, name},
};
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;

#[derive(Debug, Clone, PartialEq, Eq)]
struct TokenName(name::TokenName);

impl From<name::TokenName> for TokenName {
    fn from(val: name::TokenName) -> Self {
        Self(val)
    }
}

impl Serialize for TokenName {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        hex::encode(self.0.as_ref()).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for TokenName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let data = hex::decode(&s).map_err(<D::Error as serde::de::Error>::custom)?;
        Ok(Self(
            name::TokenName::try_from(data).map_err(<D::Error as serde::de::Error>::custom)?,
        ))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MintingPolicy(minting_policy::MintingPolicy);

impl From<minting_policy::MintingPolicy> for MintingPolicy {
    fn from(val: minting_policy::MintingPolicy) -> Self {
        Self(val)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MintToken {
    name: TokenName,
    // TODO add a serde implementation for the MintingPolicy when it will be well specified
    #[serde(skip)]
    policy: MintingPolicy,
    to: Identifier,
    value: Value,
}

impl From<certificate::MintToken> for MintToken {
    fn from(val: certificate::MintToken) -> Self {
        Self {
            name: val.name.into(),
            policy: val.policy.into(),
            to: val.to.into(),
            value: val.value.into(),
        }
    }
}

impl From<MintToken> for certificate::MintToken {
    fn from(val: MintToken) -> Self {
        Self {
            name: val.name.0,
            policy: val.policy.0,
            to: val.to.to_inner(),
            value: val.value.into(),
        }
    }
}

#[cfg(test)]
mod test {
    use quickcheck::Arbitrary;

    use super::*;

    impl Arbitrary for MintToken {
        fn arbitrary<G: quickcheck::Gen>(g: &mut G) -> Self {
            certificate::MintToken::arbitrary(g).into()
        }
    }
}
