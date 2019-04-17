extern crate serde_derive;
use self::serde_derive::{Deserialize, Serialize};
use std::vec::Vec;

#[derive(Debug, Serialize, Deserialize)]
pub struct BlockchainConfig {
    pub block0_date: i32,
    pub discrimination: String,
    pub block0_consensus: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InitialSetting {
    pub allow_account_creation: bool,
    pub slot_duration: i32,
    pub epoch_stability_depth: i32,
    pub block_version: i32,
    pub bft_leaders: Vec<String>,
    pub linear_fees: LinearFees,
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
    pub initial_setting: InitialSetting,
    pub initial_funds: Vec<Fund>,
}

impl GenesisYaml {
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
        GenesisYaml {
            blockchain_configuration: BlockchainConfig {
                block0_date: 1554185140,
                discrimination: String::from("test"),
                block0_consensus: String::from("bft"),
            },
            initial_setting: InitialSetting {
                allow_account_creation: true,
                slot_duration: 15,
                epoch_stability_depth: 2600,
                block_version: 1,
                linear_fees: LinearFees {
                    constant: 0,
                    coefficient: 0,
                    certificate: 0,
                },
                bft_leaders: vec![
                    String::from(
                        "ed25519e_pk1else5uqslegj6n5rxnrayz2x99cel6m2g492ac6tpv76kns0dwlqpjnh0l",
                    ),
                    String::from(
                        "ed25519e_pk1xuqdxht6f0kkh0lf3ck3gfyvnpk33s09du92w6740mfmxl6hsfpsp8grmk",
                    ),
                ],
            },
            initial_funds,
        }
    }
}
