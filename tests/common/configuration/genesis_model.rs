#![allow(dead_code)]

extern crate serde_derive;
use self::serde_derive::{Deserialize, Serialize};
use std::path::PathBuf;
use std::vec::Vec;

use super::file_utils;

#[derive(Debug, Serialize, Deserialize)]
pub struct BlockchainConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block0_date: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discrimination: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block0_consensus: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slot_duration: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slots_per_epoch: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub epoch_stability_depth: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub consensus_leader_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bft_slots_ratio: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub consensus_genesis_praos_param_f: Option<String>,
    pub allow_account_creation: bool,
    pub linear_fees: LinearFees,
    pub kes_update_speed: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LinearFees {
    pub constant: i32,
    pub coefficient: i32,
    pub certificate: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Fund {
    pub value: i32,
    pub address: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GenesisYaml {
    pub blockchain_configuration: BlockchainConfig,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initial_funds: Option<Vec<Fund>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub legacy_funds: Option<Vec<Fund>>,
}

impl GenesisYaml {
    pub fn serialize(genesis_yaml: &GenesisYaml) -> PathBuf {
        let content = serde_yaml::to_string(&genesis_yaml).unwrap();
        let input_yaml_file_path = file_utils::create_file_in_temp("genesis.yaml", &content);
        input_yaml_file_path
    }

    pub fn new() -> GenesisYaml {
        let initial_funds = vec![
            Fund {
                address: String::from(
                    "ta1sdz0t7tqv4etykkajvng6mscxzvzcragdq9pzd8s0x9w93n38h7gxry6rqf",
                ),
                value: 100,
            },
            Fund {
                address: String::from(
                    "ta1sd5luh6nuw6a34y5ayhhaekk6225w5667x29n9qg0nvat7k7kennj35d456",
                ),
                value: 100,
            },
        ];
        GenesisYaml::new_with_funds(initial_funds)
    }

    pub fn new_with_funds(initial_funds: Vec<Fund>) -> GenesisYaml {
        GenesisYaml::new_with_funds_and_legacy(Some(initial_funds), None)
    }

    pub fn new_with_legacy_funds(legacy_funds: Vec<Fund>) -> GenesisYaml {
        GenesisYaml::new_with_funds_and_legacy(None, Some(legacy_funds))
    }

    pub fn new_with_funds_and_legacy(
        initial_funds: Option<Vec<Fund>>,
        legacy_funds: Option<Vec<Fund>>,
    ) -> GenesisYaml {
        GenesisYaml {
            blockchain_configuration: BlockchainConfig {
                block0_date: Some(1554185140),
                discrimination: Some(String::from("test")),
                block0_consensus: Some(String::from("bft")),
                slot_duration: Some(15),
                slots_per_epoch: Some(100),
                epoch_stability_depth: Some(2600),
                consensus_leader_ids: Some(vec![
                    String::from(
                        "ed25519e_pk1else5uqslegj6n5rxnrayz2x99cel6m2g492ac6tpv76kns0dwlqpjnh0l",
                    ),
                    String::from(
                        "ed25519e_pk1xuqdxht6f0kkh0lf3ck3gfyvnpk33s09du92w6740mfmxl6hsfpsp8grmk",
                    ),
                ]),
                bft_slots_ratio: Some("0.222".to_owned()),
                consensus_genesis_praos_active_slot_coeff: "0.444".to_owned(),
                allow_account_creation: true,
                linear_fees: LinearFees {
                    constant: 0,
                    coefficient: 0,
                    certificate: 0,
                },
                kes_update_speed: 12 * 3600,
            },
            initial_funds: initial_funds,
            legacy_funds: legacy_funds,
        }
    }
}
