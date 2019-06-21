use chain_crypto::{bech32::Bech32 as _, Ed25519, PublicKey};
use chain_impl_mockchain::{config::ConfigParam, leadership::bft::LeaderId};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{convert::TryFrom, fmt};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ConsensusLeaderId(pub LeaderId);

custom_error! { pub TryFromConsensusLeaderIdError
    Incompatible = "Incompatible Config param, expected Add BFT Leader",
}

impl TryFrom<ConfigParam> for ConsensusLeaderId {
    type Error = TryFromConsensusLeaderIdError;
    fn try_from(config_param: ConfigParam) -> Result<Self, Self::Error> {
        match config_param {
            ConfigParam::AddBftLeader(leader_id) => Ok(ConsensusLeaderId(leader_id)),
            _ => Err(TryFromConsensusLeaderIdError::Incompatible),
        }
    }
}

impl From<ConsensusLeaderId> for ConfigParam {
    fn from(consensus_leader_id: ConsensusLeaderId) -> Self {
        ConfigParam::AddBftLeader(consensus_leader_id.0)
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
                    .map(|pk| ConsensusLeaderId(pk.into()))
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
            ConsensusLeaderId(LeaderId::from(public_key))
        }
    }

    #[test]
    fn deserialize_from_str() {
        const STR: &'static str =
            "---\n\"ed25519_pk1evu9kfx9tztez708nac569hcp0xwkvekxpwc7m8ztxu44tmq4gws3yayej\"";

        let _: ConsensusLeaderId = serde_yaml::from_str(&STR).unwrap();
    }

    quickcheck! {
        fn serde_encode_decode(consensus_leader_id: ConsensusLeaderId) -> bool {
            let s = serde_yaml::to_string(&consensus_leader_id).unwrap();
            let consensus_leader_id_dec: ConsensusLeaderId = serde_yaml::from_str(&s).unwrap();

            consensus_leader_id == consensus_leader_id_dec
        }

        fn convert_from_to_config_param(consensus_leader_id: ConsensusLeaderId) -> bool {
            let cp = ConfigParam::from(consensus_leader_id.clone());
            let consensus_leader_id_dec = ConsensusLeaderId::try_from(cp).unwrap();

            consensus_leader_id == consensus_leader_id_dec
        }
    }
}
