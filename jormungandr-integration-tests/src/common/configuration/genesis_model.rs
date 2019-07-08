#![allow(dead_code)]

extern crate chain_addr;
extern crate chain_crypto;
extern crate rand;
extern crate rand_chacha;
extern crate serde_derive;
use self::serde_derive::{Deserialize, Serialize};
use jormungandr_lib::interfaces::Value;
use std::path::PathBuf;
use std::vec::Vec;

use self::chain_addr::{Address, Discrimination};
use self::chain_addr::{AddressReadable, Kind};
use self::chain_crypto::bech32::Bech32;
use self::chain_crypto::{Ed25519, Ed25519Extended, KeyPair, PublicKey, SecretKey};
use self::rand::SeedableRng;
use self::rand_chacha::ChaChaRng;

use super::file_utils;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BlockchainConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block0_date: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discrimination: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block0_consensus: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slot_duration: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slots_per_epoch: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub epoch_stability_depth: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub consensus_leader_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bft_slots_ratio: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub consensus_genesis_praos_active_slot_coeff: Option<String>,
    pub linear_fees: LinearFees,
    pub kes_update_speed: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LinearFees {
    pub constant: u32,
    pub coefficient: u32,
    pub certificate: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Fund {
    pub value: Value,
    pub address: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Initial {
    Fund(Vec<Fund>),
    Cert(String),
    LegacyFund(Vec<Fund>),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GenesisYaml {
    pub blockchain_configuration: BlockchainConfig,
    pub initial: Vec<Initial>,
}

impl GenesisYaml {
    pub fn serialize(genesis_yaml: &GenesisYaml) -> PathBuf {
        let content = serde_yaml::to_string(&genesis_yaml).unwrap();
        let input_yaml_file_path = file_utils::create_file_in_temp("genesis.yaml", &content);
        input_yaml_file_path
    }

    pub fn new() -> GenesisYaml {
        let sk1: SecretKey<Ed25519Extended> =
            SecretKey::generate(&mut ChaChaRng::from_seed([1; 32]));
        let pk1: PublicKey<Ed25519> = sk1.to_public();
        let initial_funds_address1 = Address(Discrimination::Test, Kind::Single(pk1));
        let initial_funds_address1 =
            AddressReadable::from_address("ca", &initial_funds_address1).to_string();

        let sk2: SecretKey<Ed25519Extended> =
            SecretKey::generate(&mut ChaChaRng::from_seed([2; 32]));
        let pk2: PublicKey<Ed25519> = sk2.to_public();
        let initial_funds_address2 = Address(Discrimination::Test, Kind::Single(pk2));
        let initial_funds_address2 =
            AddressReadable::from_address("ca", &initial_funds_address2).to_string();

        let initial_funds = vec![
            Fund {
                address: String::from(initial_funds_address1),
                value: 100.into(),
            },
            Fund {
                address: String::from(initial_funds_address2),
                value: 100.into(),
            },
        ];
        GenesisYaml::new_with_funds(&initial_funds)
    }

    pub fn new_with_funds(initial_funds: &[Fund]) -> GenesisYaml {
        GenesisYaml::new_with_funds_and_legacy(initial_funds, &[])
    }

    pub fn new_with_legacy_funds(legacy_funds: &[Fund]) -> GenesisYaml {
        GenesisYaml::new_with_funds_and_legacy(&[], legacy_funds)
    }

    pub fn new_with_funds_and_legacy(initial_funds: &[Fund], legacy_funds: &[Fund]) -> GenesisYaml {
        let leader_1: KeyPair<Ed25519Extended> =
            KeyPair::generate(&mut ChaChaRng::from_seed([1; 32]));
        let leader_2: KeyPair<Ed25519Extended> =
            KeyPair::generate(&mut ChaChaRng::from_seed([2; 32]));
        let leader_1_pk = leader_1.public_key().to_bech32_str();
        let leader_2_pk = leader_2.public_key().to_bech32_str();
        let funds = Initial::Fund(initial_funds.iter().cloned().collect());
        let legacy = Initial::LegacyFund(legacy_funds.iter().cloned().collect());
        GenesisYaml {
            blockchain_configuration: BlockchainConfig {
                block0_date: Some(1554185140),
                discrimination: Some(String::from("test")),
                block0_consensus: Some(String::from("bft")),
                slot_duration: Some(1),
                slots_per_epoch: Some(100),
                epoch_stability_depth: Some(2600),
                consensus_leader_ids: Some(vec![
                    String::from(leader_1_pk),
                    String::from(leader_2_pk),
                ]),
                bft_slots_ratio: Some("0.222".to_owned()),
                consensus_genesis_praos_active_slot_coeff: Some("0.444".to_owned()),
                linear_fees: LinearFees {
                    constant: 0,
                    coefficient: 0,
                    certificate: 0,
                },
                kes_update_speed: 12 * 3600,
            },
            initial: vec![funds, legacy],
        }
    }
}
