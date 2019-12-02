pub mod arbitrary;
pub mod builders;
pub mod data;
pub mod ledger;
pub mod verifiers;
pub mod scenario;
pub mod e2e;
pub use arbitrary::*;
pub use builders::*;
pub use data::KeysDb;
pub use ledger::{ConfigBuilder, LedgerBuilder, TestLedger, UtxoDb};

pub use chain_crypto::testing::TestCryptoGen;

use crate::key::Hash;
use crate::{
    account::Identifier,
    config::ConfigParam,
    fragment::config::ConfigParams,
    leadership::bft::LeaderId,
    quickcheck::RngCore,
    setting::Settings,
    testing::data::{AddressData, LeaderPair},
};

use chain_crypto::{Ed25519, Ed25519Extended, PublicKey, KeyPair};
use std::iter;

pub struct TestGen;

impl TestGen {
    pub fn hash() -> Hash {
        Hash::from_bytes(Self::bytes())
    }

    pub fn bytes() -> [u8;32] {
        let mut random_bytes: [u8; 32] = [0; 32];
        rand_os::OsRng::new().unwrap().fill_bytes(&mut random_bytes);
        random_bytes
    }

    pub fn identifier() -> Identifier {
        let kp: KeyPair<Ed25519> = AddressData::generate_key_pair::<Ed25519>();
        Identifier::from(kp.into_keys().1)
    }

    pub fn public_key() -> PublicKey<Ed25519> {
        AddressData::generate_key_pair::<Ed25519>().public_key().clone()
    }

    pub fn leader_pair() -> LeaderPair {
        let leader_key = AddressData::generate_key_pair::<Ed25519Extended>()
            .private_key()
            .clone();
        let leader_id = LeaderId(
            AddressData::generate_key_pair::<Ed25519>()
                .public_key()
                .clone(),
        );
        LeaderPair::new(leader_id, leader_key)
    }

    pub fn leaders_pairs() -> impl Iterator<Item = LeaderPair> {
        iter::from_fn(|| Some(TestGen::leader_pair()))
    }

    pub fn settings(leaders: Vec<LeaderPair>) -> Settings {
        let settings = Settings::new();
        let mut config_params = ConfigParams::new();
        for leader_id in leaders.iter().cloned().map(|x| x.id()) {
            config_params.push(ConfigParam::AddBftLeader(leader_id));
        }
        settings.apply(&config_params).unwrap()
    }
}
