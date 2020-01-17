#![allow(dead_code)]

extern crate chain_addr;
extern crate chain_crypto;
extern crate chain_impl_mockchain;
extern crate rand;
extern crate rand_chacha;
extern crate serde_derive;
use self::chain_addr::{Address as ChainAddress, Discrimination, Kind};
use self::chain_crypto::{Ed25519, Ed25519Extended, KeyPair, PublicKey, SecretKey};
use self::rand::SeedableRng;
use self::rand_chacha::ChaChaRng;
use self::serde_derive::{Deserialize, Serialize};
use super::file_utils;
use chain_impl_mockchain::{block::ConsensusVersion, fee::LinearFee};
use jormungandr_lib::{
    interfaces::{
        ActiveSlotCoefficient, Address, BlockchainConfiguration, ConsensusLeaderId, Initial,
        InitialUTxO, KESUpdateSpeed, LegacyUTxO, NumberOfSlotsPerEpoch, Ratio, RewardConstraints,
        RewardParams, SlotDuration, TaxType,
    },
    time::SecondsSinceUnixEpoch,
};

use std::num::NonZeroU32;
use std::path::PathBuf;
use std::vec::Vec;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GenesisYaml {
    pub blockchain_configuration: BlockchainConfiguration,
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
        let leader_1_pk = leader_1.public_key();
        let leader_2_pk = leader_2.public_key();

        let mut initial = Vec::new();
        if initial_funds.len() > 0 {
            initial.push(Initial::Fund(initial_funds.iter().cloned().collect()))
        }
        if legacy_funds.len() > 0 {
            initial.push(Initial::LegacyFund(legacy_funds.iter().cloned().collect()))
        }

        let mut consensus_leader_ids: Vec<ConsensusLeaderId> = Vec::new();
        consensus_leader_ids.push(leader_1_pk.clone().into());
        consensus_leader_ids.push(leader_2_pk.clone().into());

        GenesisYaml {
            blockchain_configuration: BlockchainConfiguration {
                block_content_max_size: 4096.into(),
                fees_go_to: None,
                reward_constraints: RewardConstraints {
                    reward_drawing_limit_max: None,
                    pool_participation_capping: None,
                },
                block0_date: SecondsSinceUnixEpoch::now(),
                discrimination: Discrimination::Test,
                block0_consensus: ConsensusVersion::Bft,
                slot_duration: SlotDuration::new(1u8).unwrap(),
                slots_per_epoch: NumberOfSlotsPerEpoch::new(100u32).unwrap(),
                epoch_stability_depth: 2600u32.into(),
                consensus_leader_ids: consensus_leader_ids,
                consensus_genesis_praos_active_slot_coeff: ActiveSlotCoefficient::MAXIMUM,
                linear_fees: LinearFee::new(0, 0, 0),
                kes_update_speed: KESUpdateSpeed::new(12 * 3600).unwrap(),
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
