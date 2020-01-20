#![allow(dead_code)]

extern crate chain_addr;
extern crate chain_crypto;
extern crate chain_impl_mockchain;
extern crate rand;
extern crate rand_chacha;
extern crate serde_derive;
use self::chain_addr::{Address as ChainAddress, Discrimination, Kind};
use self::chain_crypto::bech32::Bech32;
use self::chain_crypto::{Ed25519, Ed25519Extended, KeyPair, PublicKey, SecretKey};
use self::chain_impl_mockchain::fee::LinearFee;
use self::rand::SeedableRng;
use self::rand_chacha::ChaChaRng;
use self::serde_derive::{Deserialize, Serialize};
use super::file_utils;
use jormungandr_lib::interfaces::{
    Address, Initial, InitialUTxO, LegacyUTxO, LinearFeeDef, Ratio, RewardParams, TaxType, Value,
};
use std::num::NonZeroU32;
use std::path::PathBuf;
use std::vec::Vec;

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
    pub consensus_genesis_praos_active_slot_coeff: Option<String>,
    #[serde(with = "LinearFeeDef")]
    pub linear_fees: LinearFee,
    pub kes_update_speed: u32,
    #[serde(default)]
    pub treasury: Option<Value>,
    #[serde(default)]
    pub treasury_parameters: Option<TaxType>,
    #[serde(default)]
    pub total_reward_supply: Option<Value>,
    #[serde(default)]
    pub reward_parameters: Option<RewardParams>,
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
        let prefix = "ca".to_owned();
        let sk1: SecretKey<Ed25519Extended> =
            SecretKey::generate(&mut ChaChaRng::from_seed([1; 32]));
        let pk1: PublicKey<Ed25519> = sk1.to_public();
        let initial_funds_address1 = ChainAddress(Discrimination::Test, Kind::Single(pk1));

        let sk2: SecretKey<Ed25519Extended> =
            SecretKey::generate(&mut ChaChaRng::from_seed([2; 32]));
        let pk2: PublicKey<Ed25519> = sk2.to_public();
        let initial_funds_address2 = ChainAddress(Discrimination::Test, Kind::Single(pk2));
        let initial_funds = vec![
            InitialUTxO {
                address: Address(prefix.clone(), initial_funds_address1),
                value: 100.into(),
            },
            InitialUTxO {
                address: Address(prefix.clone(), initial_funds_address2),
                value: 100.into(),
            },
        ];
        GenesisYaml::new_with_funds(&initial_funds)
    }

    pub fn new_with_funds(initial_funds: &[InitialUTxO]) -> GenesisYaml {
        GenesisYaml::new_with_funds_and_legacy(initial_funds, &[])
    }

    pub fn new_with_legacy_funds(legacy_funds: &[LegacyUTxO]) -> GenesisYaml {
        GenesisYaml::new_with_funds_and_legacy(&[], legacy_funds)
    }

    pub fn new_with_funds_and_legacy(
        initial_funds: &[InitialUTxO],
        legacy_funds: &[LegacyUTxO],
    ) -> GenesisYaml {
        let leader_1: KeyPair<Ed25519Extended> =
            KeyPair::generate(&mut ChaChaRng::from_seed([1; 32]));
        let leader_2: KeyPair<Ed25519Extended> =
            KeyPair::generate(&mut ChaChaRng::from_seed([2; 32]));
        let leader_1_pk = leader_1.public_key().to_bech32_str();
        let leader_2_pk = leader_2.public_key().to_bech32_str();

        let mut initial = Vec::new();
        if initial_funds.len() > 0 {
            initial.push(Initial::Fund(initial_funds.iter().cloned().collect()))
        }
        if legacy_funds.len() > 0 {
            initial.push(Initial::LegacyFund(legacy_funds.iter().cloned().collect()))
        }

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
                consensus_genesis_praos_active_slot_coeff: Some("0.444".to_owned()),
                linear_fees: LinearFee::new(0, 0, 0),
                kes_update_speed: 12 * 3600,
                treasury: Some(1_000_000.into()),
                treasury_parameters: Some(TaxType {
                    fixed: 10.into(),
                    ratio: Ratio::new_checked(1, 1_000).unwrap(),
                    max_limit: None,
                }),
                total_reward_supply: Some(1_000_000_000.into()),
                reward_parameters: Some(RewardParams::Linear {
                    constant: 100_000,
                    ratio: Ratio::new_checked(1, 1_00).unwrap(),
                    epoch_start: 0,
                    epoch_rate: NonZeroU32::new(1).unwrap(),
                }),
            },
            initial,
        }
    }
}
