use crate::crypto::key::Identifier;
use chain_crypto::{bech32::Bech32 as _, Ed25519, PublicKey};
use chain_impl_mockchain::{
    config::ConfigParam,
    key::{BftLeaderId, BftVerificationAlg},
};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ConsensusLeaderId(BftLeaderId);

impl From<ConsensusLeaderId> for ConfigParam {
    fn from(consensus_leader_id: ConsensusLeaderId) -> Self {
        ConfigParam::AddBftLeader(consensus_leader_id.0)
    }
}

impl From<Identifier<Ed25519>> for ConsensusLeaderId {
    fn from(identifier: Identifier<Ed25519>) -> Self {
        ConsensusLeaderId(BftLeaderId::from(identifier.into_public_key()))
    }
}

impl From<PublicKey<Ed25519>> for ConsensusLeaderId {
    fn from(public_key: PublicKey<Ed25519>) -> Self {
        ConsensusLeaderId(BftLeaderId::from(public_key))
    }
}

impl From<BftLeaderId> for ConsensusLeaderId {
    fn from(leader_id: BftLeaderId) -> Self {
        Self(leader_id)
    }
}

impl From<ConsensusLeaderId> for BftLeaderId {
    fn from(leader_id: ConsensusLeaderId) -> Self {
        leader_id.0
    }
}

impl Serialize for ConsensusLeaderId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.as_public_key().to_bech32_str().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ConsensusLeaderId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::{self, Visitor};
        struct ConsensusLeaderIdVisitor;
        impl<'de> Visitor<'de> for ConsensusLeaderIdVisitor {
            type Value = ConsensusLeaderId;
            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                use chain_crypto::AsymmetricPublicKey as _;
                write!(
                    formatter,
                    "bech32 encoding of the leader id's public key ({})",
                    Ed25519::PUBLIC_BECH32_HRP
                )
            }

            fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                PublicKey::try_from_bech32_str(s)
                    .map(|pk: chain_crypto::PublicKey<BftVerificationAlg>| {
                        ConsensusLeaderId(pk.into())
                    })
                    .map_err(E::custom)
            }
        }

        deserializer.deserialize_str(ConsensusLeaderIdVisitor)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use quickcheck::{Arbitrary, Gen};

    impl Arbitrary for ConsensusLeaderId {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            use crate::crypto::key::KeyPair;

            let kp: KeyPair<Ed25519> = KeyPair::arbitrary(g);
            let public_key = kp.identifier().into_public_key();
            ConsensusLeaderId(BftLeaderId::from(public_key))
        }
    }

    #[test]
    fn deserialize_from_str() {
        const STR: &str =
            "---\n\"ed25519_pk1evu9kfx9tztez708nac569hcp0xwkvekxpwc7m8ztxu44tmq4gws3yayej\"";

        let _: ConsensusLeaderId = serde_yaml::from_str(STR).unwrap();
    }

    quickcheck! {
        fn serde_encode_decode(consensus_leader_id: ConsensusLeaderId) -> bool {
            let s = serde_yaml::to_string(&consensus_leader_id).unwrap();
            let consensus_leader_id_dec: ConsensusLeaderId = serde_yaml::from_str(&s).unwrap();

            consensus_leader_id == consensus_leader_id_dec
        }
    }
}
