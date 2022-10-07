#![allow(dead_code)]

use chain_addr::{Address as ChainAddress, Discrimination, Kind};
use chain_crypto::{Ed25519, Ed25519Extended, PublicKey, SecretKey};
use chain_impl_mockchain::{chaintypes::ConsensusVersion, fee::LinearFee};
use jormungandr_lib::{
    crypto::key::{KeyPair, SigningKey},
    interfaces::{
        ActiveSlotCoefficient, Block0Configuration, BlockContentMaxSize, BlockchainConfiguration,
        CommitteeIdDef, ConsensusLeaderId, EpochStabilityDepth, FeesGoTo, Initial, InitialToken,
        InitialUTxO, KesUpdateSpeed, NumberOfSlotsPerEpoch, ProposalExpiration, Ratio,
        RewardConstraints, RewardParams, SignedCertificate, SlotDuration, TaxType, Value,
    },
    time::SecondsSinceUnixEpoch,
};
use rand::SeedableRng;
use rand_chacha::ChaChaRng;
use serde_derive::{Deserialize, Serialize};
use std::{
    num::{NonZeroU32, NonZeroU64},
    vec::Vec,
};

const DEFAULT_SLOT_DURATION: u8 = 2;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Block0ConfigurationBuilder {
    pub blockchain_configuration: BlockchainConfiguration,
    pub initial: Vec<Initial>,
    minimal_setup: bool,
}

impl Default for Block0ConfigurationBuilder {
    fn default() -> Self {
        Self {
            initial: Vec::new(),
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
                slot_duration: SlotDuration::new(2u8).unwrap(),
                slots_per_epoch: NumberOfSlotsPerEpoch::new(100u32).unwrap(),
                epoch_stability_depth: 2600u32.into(),
                consensus_leader_ids: vec![],
                consensus_genesis_praos_active_slot_coeff: ActiveSlotCoefficient::MINIMUM,
                linear_fees: LinearFee::new(0, 0, 0),
                proposal_expiration: ProposalExpiration::default(),
                kes_update_speed: KesUpdateSpeed::new(12 * 3600).unwrap(),
                treasury: Some(1_000_000.into()),
                treasury_parameters: Some(TaxType {
                    fixed: 10.into(),
                    ratio: Ratio::new_checked(1, 1_000).unwrap(),
                    max_limit: NonZeroU64::new(123),
                }),
                total_reward_supply: Some(1_000_000_000.into()),
                reward_parameters: Some(RewardParams::Linear {
                    constant: 100_000,
                    ratio: Ratio::new_checked(1, 1_00).unwrap(),
                    epoch_start: 0,
                    epoch_rate: NonZeroU32::new(1).unwrap(),
                }),
                committees: Vec::new(),
                tx_max_expiry_epochs: Some(100),
                #[cfg(feature = "evm")]
                evm_configs: None,
                #[cfg(feature = "evm")]
                evm_env_settings: None,
            },
            minimal_setup: false,
        }
    }
}

impl Block0ConfigurationBuilder {
    pub fn minimal_setup() -> Self {
        let block0_configuration_builder = Self::default();
        let discrimination = block0_configuration_builder
            .blockchain_configuration
            .discrimination;
        block0_configuration_builder
            .with_consensus_leaders_ids(default_leaders())
            .with_initial(default_initial(discrimination))
    }

    pub fn with_signed_certs(mut self, signed_certs: Vec<SignedCertificate>) -> Self {
        for signed_cert in signed_certs {
            self.initial.push(Initial::Cert(signed_cert));
        }
        self
    }

    pub fn with_funds_split_if_needed(self, utxos: Vec<InitialUTxO>) -> Self {
        self.with_funds_split_if_needed_recursively(&utxos)
    }

    pub fn with_funds_split_if_needed_recursively(self, utxos: &[InitialUTxO]) -> Self {
        if utxos.is_empty() {
            return self;
        }
        if utxos.len() < 255 {
            return self.with_utxos(utxos.to_vec());
        }

        let (left, right) = utxos.split_at(255);
        self.with_funds_split_if_needed_recursively(left)
            .with_funds_split_if_needed_recursively(right)
    }

    pub fn with_some_consensus_leader(self) -> Self {
        let leader_key = KeyPair::generate(&mut rand::thread_rng());
        self.with_leader_key_pair(&leader_key)
    }

    pub fn with_utxo(self, utxo: InitialUTxO) -> Self {
        self.with_utxos(vec![utxo])
    }

    pub fn with_leader_key_pair(mut self, leader_pair: &KeyPair<Ed25519>) -> Self {
        self.blockchain_configuration
            .consensus_leader_ids
            .push(leader_pair.0.public_key().clone().into());
        self
    }

    pub fn with_leader_signing_key(mut self, signing_key: SigningKey<Ed25519>) -> Self {
        self.blockchain_configuration
            .consensus_leader_ids
            .push(signing_key.identifier().into());
        self
    }

    pub fn with_leader(mut self, leader: ConsensusLeaderId) -> Self {
        self.blockchain_configuration
            .consensus_leader_ids
            .push(leader);
        self
    }

    pub fn with_committees(mut self, committees: &[CommitteeIdDef]) -> Self {
        self.blockchain_configuration.committees = committees.to_vec();
        self
    }

    pub fn with_consensus_genesis_praos_active_slot_coeff(
        mut self,
        active_slot_coeff: ActiveSlotCoefficient,
    ) -> Self {
        self.blockchain_configuration
            .consensus_genesis_praos_active_slot_coeff = active_slot_coeff;
        self
    }

    pub fn with_funds(mut self, funds: Vec<Initial>) -> Self {
        self.initial.extend(funds.iter().cloned());
        self
    }

    pub fn with_utxos(mut self, utxos: Vec<InitialUTxO>) -> Self {
        self.initial.push(Initial::Fund(utxos));
        self
    }

    pub fn with_token(mut self, initial_token: InitialToken) -> Self {
        self.initial.push(Initial::Token(initial_token));
        self
    }

    pub fn with_block_content_max_size(
        mut self,
        block_content_max_size: BlockContentMaxSize,
    ) -> Self {
        self.blockchain_configuration.block_content_max_size = block_content_max_size;
        self
    }

    pub fn with_consensus_leaders_ids(mut self, leaders_ids: Vec<ConsensusLeaderId>) -> Self {
        self.blockchain_configuration
            .consensus_leader_ids
            .extend(leaders_ids);
        self
    }

    pub fn with_block0_consensus(mut self, block0_consensus: ConsensusVersion) -> Self {
        self.blockchain_configuration.block0_consensus = block0_consensus;
        self
    }

    pub fn with_kes_update_speed(mut self, kes_update_speed: KesUpdateSpeed) -> Self {
        self.blockchain_configuration.kes_update_speed = kes_update_speed;
        self
    }

    pub fn with_slots_per_epoch(mut self, slots_per_epoch: NumberOfSlotsPerEpoch) -> Self {
        self.blockchain_configuration.slots_per_epoch = slots_per_epoch;
        self
    }

    pub fn with_slot_duration(mut self, slot_duration: SlotDuration) -> Self {
        self.blockchain_configuration.slot_duration = slot_duration;
        self
    }

    pub fn with_discrimination(mut self, discrimination: Discrimination) -> Self {
        self.blockchain_configuration.discrimination = discrimination;
        self
    }

    pub fn with_epoch_stability_depth(
        mut self,
        epoch_stability_depth: EpochStabilityDepth,
    ) -> Self {
        self.blockchain_configuration.epoch_stability_depth = epoch_stability_depth;
        self
    }

    pub fn with_active_slot_coeff(
        mut self,
        consensus_genesis_praos_active_slot_coeff: ActiveSlotCoefficient,
    ) -> Self {
        self.blockchain_configuration
            .consensus_genesis_praos_active_slot_coeff = consensus_genesis_praos_active_slot_coeff;
        self
    }

    pub fn with_treasury(mut self, treasury: Value) -> Self {
        self.blockchain_configuration.treasury = Some(treasury);
        self
    }

    pub fn with_reward_parameters(mut self, reward_parameters: Option<RewardParams>) -> Self {
        self.blockchain_configuration.reward_parameters = reward_parameters;
        self
    }

    pub fn with_total_rewards_supply(mut self, total_reward_supply: Option<Value>) -> Self {
        self.blockchain_configuration.total_reward_supply = total_reward_supply;
        self
    }

    pub fn with_committee_ids(mut self, committee_ids: Vec<CommitteeIdDef>) -> Self {
        self.blockchain_configuration.committees = committee_ids;
        self
    }

    pub fn with_linear_fees(mut self, linear_fees: LinearFee) -> Self {
        self.blockchain_configuration.linear_fees = linear_fees;
        self
    }

    pub fn with_proposal_expiration(mut self, proposal_expiration: ProposalExpiration) -> Self {
        self.blockchain_configuration.proposal_expiration = proposal_expiration;
        self
    }

    pub fn with_certs(mut self, certs: Vec<Initial>) -> Self {
        self.initial.extend(certs.iter().cloned());
        self
    }

    pub fn with_initial(mut self, initial: Vec<Initial>) -> Self {
        self.initial.extend(initial.iter().cloned());
        self
    }

    pub fn with_fees_go_to(mut self, fees_go_to: Option<FeesGoTo>) -> Self {
        self.blockchain_configuration.fees_go_to = fees_go_to;
        self
    }

    pub fn with_treasury_parameters(mut self, treasury_parameters: Option<TaxType>) -> Self {
        self.blockchain_configuration.treasury_parameters = treasury_parameters;
        self
    }

    pub fn with_tx_max_expiry_epochs(mut self, tx_max_expiry_epochs: u8) -> Self {
        self.blockchain_configuration.tx_max_expiry_epochs = Some(tx_max_expiry_epochs);
        self
    }

    pub fn build(mut self) -> Block0Configuration {
        if self.minimal_setup {
            if self.initial.is_empty() {
                self.initial.extend(
                    default_initial(self.blockchain_configuration.discrimination)
                        .iter()
                        .cloned(),
                );
            }

            if self
                .blockchain_configuration
                .consensus_leader_ids
                .is_empty()
            {
                self.blockchain_configuration.consensus_leader_ids = default_leaders();
            }
        }
        Block0Configuration {
            blockchain_configuration: self.blockchain_configuration.clone(),
            initial: self.initial,
        }
    }
}

fn default_initial(discrimination: Discrimination) -> Vec<Initial> {
    let sk1: SecretKey<Ed25519Extended> = SecretKey::generate(&mut ChaChaRng::from_seed([1; 32]));
    let pk1: PublicKey<Ed25519> = sk1.to_public();
    let initial_funds_address1 = ChainAddress(discrimination, Kind::Single(pk1));

    let sk2: SecretKey<Ed25519Extended> = SecretKey::generate(&mut ChaChaRng::from_seed([2; 32]));
    let pk2: PublicKey<Ed25519> = sk2.to_public();
    let initial_funds_address2 = ChainAddress(discrimination, Kind::Single(pk2));
    let initial_funds = vec![Initial::Fund(vec![
        InitialUTxO {
            address: initial_funds_address1.into(),
            value: 100.into(),
        },
        InitialUTxO {
            address: initial_funds_address2.into(),
            value: 100.into(),
        },
    ])];
    initial_funds
}

fn default_leaders() -> Vec<ConsensusLeaderId> {
    let leader_1: KeyPair<Ed25519Extended> = KeyPair::generate(&mut ChaChaRng::from_seed([1; 32]));
    let leader_2: KeyPair<Ed25519Extended> = KeyPair::generate(&mut ChaChaRng::from_seed([2; 32]));
    let mut leaders: Vec<ConsensusLeaderId> = Vec::new();
    let leader_1_pk: ConsensusLeaderId = leader_1.0.public_key().clone().into();
    let leader_2_pk: ConsensusLeaderId = leader_2.0.public_key().clone().into();
    leaders.push(leader_1_pk);
    leaders.push(leader_2_pk);
    leaders
}
