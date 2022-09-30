use crate::{
    interfaces::{
        ActiveSlotCoefficient, BlockContentMaxSize, CommitteeIdDef, ConsensusLeaderId,
        EpochStabilityDepth, FeesGoTo, KesUpdateSpeed, LinearFeeDef, NumberOfSlotsPerEpoch,
        PoolParticipationCapping, ProposalExpiration, RewardConstraints, RewardParams,
        SlotDuration, TaxType, Value,
    },
    time::SecondsSinceUnixEpoch,
};
use chain_addr::Discrimination;
use chain_impl_mockchain::{
    chaintypes::ConsensusVersion,
    config::{Block0Date, ConfigParam},
    fee::LinearFee,
    fragment::config::ConfigParams,
    vote::CommitteeId,
};
use serde::{Deserialize, Serialize};
use std::convert::{TryFrom, TryInto};
use thiserror::Error;

/// Initial blockchain configuration for block0
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BlockchainConfiguration {
    /// the number of seconds since UNIX Epoch
    ///
    /// any value between 0 (1/1/1970) and 1099511627775 (20/08/4147) is valid
    #[serde(default)]
    pub block0_date: SecondsSinceUnixEpoch,

    /// the address discrimination (test or production)
    #[serde(with = "DiscriminationDef")]
    pub discrimination: Discrimination,

    /// the type of consensus to utilise from the starting point of the
    /// blockchain. `bft` or `genesis`
    #[serde(with = "ConsensusVersionDef")]
    pub block0_consensus: ConsensusVersion,

    /// the list of consensus leaders
    ///
    /// depending of `block0_consensus` value:
    ///
    /// * `bft`: will be the list of BFT leaders, they will write blocks
    ///    in a round robin fashion, filling every blocks deterministically.
    /// * `genesis`: will be the list of leaders that will take over creating
    ///   blocks from the stake pool. Useful for during transition from BFT
    ///   to genesis.
    ///
    /// If the `consensus_version` is `bft`. This value cannot be left empty.
    #[serde(default)]
    pub consensus_leader_ids: Vec<ConsensusLeaderId>,

    /// the linear fee settings.
    ///
    /// * constant is the minimal fee to pay for any kind of transaction
    /// * coefficient will be added for every inputs and outputs
    /// * certificate will be added if a certificate is embedded
    ///
    /// `constant + coefficient * (num_inputs + num_outputs) [+ certificate]`
    ///
    #[serde(with = "LinearFeeDef")]
    pub linear_fees: LinearFee,

    /// the proposal expiration settings. The default value is `100`.
    ///
    #[serde(default)]
    pub proposal_expiration: ProposalExpiration,

    /// number of slots in one given epoch. The default value is `720`.
    ///
    #[serde(default)]
    pub slots_per_epoch: NumberOfSlotsPerEpoch,

    /// the number of seconds between the creation of 2 slots. The default
    /// is `5` seconds.
    #[serde(default)]
    pub slot_duration: SlotDuration,

    /// number of seconds between 2 required KES Key updates.
    ///
    /// KES means, Key Evolving Signature. It is the scheme used in
    /// genesis to sign blocks and guarantee that one block signer
    /// cannot reuse a key that was valid at a given state when
    /// to create a fork.
    #[serde(default)]
    pub kes_update_speed: KesUpdateSpeed,

    /// the active slot coefficient to determine the minimal stake
    /// in order to participate to the consensus.
    ///
    /// default value is 0.1
    #[serde(default)]
    pub consensus_genesis_praos_active_slot_coeff: ActiveSlotCoefficient,

    /// set the block content maximal size
    #[serde(default)]
    pub block_content_max_size: BlockContentMaxSize,

    /// set the maximal depth from which a fork will no longer be considered valid
    #[serde(default)]
    pub epoch_stability_depth: EpochStabilityDepth,

    /// set the maximum number of epochs a transaction can reside in the mempool
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tx_max_expiry_epochs: Option<u8>,

    /// Fees go to settings, the default being `rewards`.
    ///
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fees_go_to: Option<FeesGoTo>,

    /// Set the default value in the treasury. if omitted then the treasury starts with the value of 0
    #[serde(skip_serializing_if = "Option::is_none")]
    pub treasury: Option<Value>,

    /// set the treasure parameters, i.e. the first value the treasury will take from the
    /// rewards pot and fees.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub treasury_parameters: Option<TaxType>,

    /// Set the value of the reward pot. if omitted then the reward pot is empty
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_reward_supply: Option<Value>,

    /// The reward settings for the reward policy. No reward settings means no reward
    /// distributed at all.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reward_parameters: Option<RewardParams>,

    #[serde(default)]
    #[serde(skip_serializing_if = "RewardConstraints::is_none")]
    pub reward_constraints: RewardConstraints,

    /// the committee members for the voting management
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub committees: Vec<CommitteeIdDef>,

    #[cfg(feature = "evm")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evm_configs: Option<crate::interfaces::evm_params::EvmConfig>,

    #[cfg(feature = "evm")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evm_env_settings: Option<crate::interfaces::evm_params::EvmEnvSettings>,
}

impl From<BlockchainConfiguration> for ConfigParams {
    fn from(blockchain_configuration: BlockchainConfiguration) -> Self {
        blockchain_configuration.into_config_params()
    }
}

type StaticStr = &'static str;

#[derive(Debug, Error)]
pub enum FromConfigParamsError {
    #[error("initial message misses parameter {name}")]
    InitConfigParamMissing { name: StaticStr },
    #[error("initial message contains duplicate parameter {name}")]
    InitConfigParamDuplicate { name: StaticStr },
    #[error("Invalid number of slots per epoch")]
    NumberOfSlotsPerEpoch(
        #[from] super::number_of_slots_per_epoch::TryFromNumberOfSlotsPerEpochError,
    ),
    #[error("Invalid slot duration value")]
    SlotDuration(#[from] super::slots_duration::TryFromSlotDurationError),
    #[error("Invalid active slot coefficient value")]
    ActiveSlotCoefficient(
        #[from] super::active_slot_coefficient::TryFromActiveSlotCoefficientError,
    ),
    #[error("Invalid KES Update speed value")]
    KesUpdateSpeed(#[from] super::kes_update_speed::TryFromKesUpdateSpeedError),
    #[error("Invalid FeesGoTo setting")]
    FeesGoTo(#[from] super::fees_go_to::TryFromFeesGoToError),
}

impl TryFrom<ConfigParams> for BlockchainConfiguration {
    type Error = FromConfigParamsError;
    fn try_from(params: ConfigParams) -> Result<Self, Self::Error> {
        Self::from_config_params(params)
    }
}

impl BlockchainConfiguration {
    pub fn new(
        discrimination: Discrimination,
        block0_consensus: ConsensusVersion,
        linear_fees: LinearFee,
    ) -> Self {
        BlockchainConfiguration {
            block0_date: SecondsSinceUnixEpoch::default(),
            discrimination,
            block0_consensus,
            linear_fees,
            proposal_expiration: ProposalExpiration::default(),
            consensus_leader_ids: Vec::default(),
            slots_per_epoch: NumberOfSlotsPerEpoch::default(),
            slot_duration: SlotDuration::default(),
            kes_update_speed: KesUpdateSpeed::default(),
            consensus_genesis_praos_active_slot_coeff: ActiveSlotCoefficient::default(),
            block_content_max_size: BlockContentMaxSize::default(),
            epoch_stability_depth: EpochStabilityDepth::default(),
            tx_max_expiry_epochs: None,
            fees_go_to: None,
            treasury: None,
            treasury_parameters: None,
            total_reward_supply: None,
            reward_parameters: None,
            reward_constraints: RewardConstraints::default(),
            committees: Vec::new(),
            #[cfg(feature = "evm")]
            evm_configs: None,
            #[cfg(feature = "evm")]
            evm_env_settings: None,
        }
    }

    fn from_config_params(params: ConfigParams) -> Result<Self, FromConfigParamsError> {
        fn param_missing_error(name: &'static str) -> FromConfigParamsError {
            FromConfigParamsError::InitConfigParamMissing { name }
        }

        let mut block0_date = None;
        let mut discrimination = None;
        let mut block0_consensus = None;
        let mut slots_per_epoch = None;
        let mut slot_duration = None;
        let mut epoch_stability_depth = None;
        let mut consensus_leader_ids = vec![];
        let mut consensus_genesis_praos_active_slot_coeff = None;
        let mut block_content_max_size = None;
        let mut linear_fees = None;
        let mut proposal_expiration = None;
        let mut kes_update_speed = None;
        let mut treasury = None;
        let mut treasury_parameters = None;
        let mut total_reward_supply = None;
        let mut reward_parameters = None;
        let mut per_certificate_fees = None;
        let mut per_vote_certificate_fees = None;
        let mut fees_go_to = None;
        let mut reward_constraints = RewardConstraints::default();
        let mut committees = Vec::new();
        let mut tx_max_expiry_epochs = None;
        #[cfg(feature = "evm")]
        let mut evm_configs = None;
        #[cfg(feature = "evm")]
        let mut evm_env_settings = None;

        for param in params.iter().cloned() {
            match param {
                ConfigParam::Block0Date(param) => block0_date
                    .replace(SecondsSinceUnixEpoch(param.0))
                    .map(|_| "block0_date"),
                ConfigParam::ConsensusVersion(param) => {
                    block0_consensus.replace(param).map(|_| "block0_consensus")
                }
                ConfigParam::Discrimination(param) => {
                    discrimination.replace(param).map(|_| "discrimination")
                }
                cp @ ConfigParam::SlotsPerEpoch(_) => slots_per_epoch
                    .replace(cp.try_into()?)
                    .map(|_| "slots_per_epoch"),
                cp @ ConfigParam::SlotDuration(_) => slot_duration
                    .replace(cp.try_into()?)
                    .map(|_| "slot_duration"),
                ConfigParam::AddBftLeader(val) => {
                    consensus_leader_ids.push(val.into());
                    None
                }
                cp @ ConfigParam::ConsensusGenesisPraosActiveSlotsCoeff(_) => {
                    consensus_genesis_praos_active_slot_coeff
                        .replace(cp.try_into()?)
                        .map(|_| "consensus_genesis_praos_active_slot_coeff")
                }
                ConfigParam::LinearFee(param) => linear_fees.replace(param).map(|_| "linear_fees"),
                cp @ ConfigParam::KesUpdateSpeed(_) => kes_update_speed
                    .replace(cp.try_into()?)
                    .map(|_| "kes_update_speed"),
                cp @ ConfigParam::FeesInTreasury(_) => {
                    fees_go_to.replace(cp.try_into()?).map(|_| "fees_go_to")
                }

                ConfigParam::RemoveBftLeader(_) => {
                    panic!("block 0 attempts to remove a BFT leader")
                }
                ConfigParam::ProposalExpiration(param) => proposal_expiration
                    .replace(param.into())
                    .map(|_| "proposal_expiration"),
                ConfigParam::BlockContentMaxSize(param) => block_content_max_size
                    .replace(param.into())
                    .map(|_| "block_content_max_size"),
                ConfigParam::EpochStabilityDepth(param) => epoch_stability_depth
                    .replace(param.into())
                    .map(|_| "epoch_stability_depth"),
                ConfigParam::TreasuryAdd(param) => {
                    treasury.replace(param.into()).map(|_| "treasury")
                }
                ConfigParam::TreasuryParams(param) => treasury_parameters
                    .replace(param.into())
                    .map(|_| "treasury_parameters"),
                ConfigParam::RewardPot(param) => total_reward_supply
                    .replace(param.into())
                    .map(|_| "total_reward_supply"),
                ConfigParam::RewardParams(param) => reward_parameters
                    .replace(param.into())
                    .map(|_| "reward_parameters"),
                ConfigParam::RewardLimitNone => {
                    panic!("ConfigParam::RewardLimitNone should not be in the block0")
                }
                ConfigParam::RewardLimitByAbsoluteStake(ratio) => reward_constraints
                    .reward_drawing_limit_max
                    .replace(ratio.into())
                    .map(|_| "reward_constraints.reward_drawing_limit_max"),
                ConfigParam::PoolRewardParticipationCapping((min, max)) => reward_constraints
                    .pool_participation_capping
                    .replace(PoolParticipationCapping { min, max })
                    .map(|_| "reward_constraints.pool_participation_capping"),
                ConfigParam::PerCertificateFees(param) => per_certificate_fees
                    .replace(param)
                    .map(|_| "per_certificate_fees"),
                ConfigParam::PerVoteCertificateFees(param) => per_vote_certificate_fees
                    .replace(param)
                    .map(|_| "per_vote_certificate_fees"),
                ConfigParam::AddCommitteeId(committee_id) => {
                    committees.push(committee_id.into());
                    None
                }
                ConfigParam::RemoveCommitteeId(_committee_id) => {
                    panic!("attempt to remove a committee in the block0")
                }
                ConfigParam::TransactionMaxExpiryEpochs(value) => tx_max_expiry_epochs
                    .replace(value)
                    .map(|_| "tx_max_expiry_epochs"),
                #[cfg(feature = "evm")]
                ConfigParam::EvmConfiguration(params) => {
                    evm_configs.replace(params.into()).map(|_| "evm_params")
                }
                #[cfg(feature = "evm")]
                ConfigParam::EvmEnvironment(params) => evm_env_settings
                    .replace(params.into())
                    .map(|_| "evm_evn_settings"),
            }
            .map(|name| Err(FromConfigParamsError::InitConfigParamDuplicate { name }))
            .unwrap_or(Ok(()))?;
        }

        if let Some(linear_fees) = &mut linear_fees {
            if let Some(per_certificate_fees) = per_certificate_fees {
                linear_fees.per_certificate_fees(per_certificate_fees);
            }

            if let Some(per_vote_certificate_fees) = per_vote_certificate_fees {
                linear_fees.per_vote_certificate_fees(per_vote_certificate_fees);
            }
        }

        Ok(BlockchainConfiguration {
            block0_date: block0_date.ok_or_else(|| param_missing_error("block0_date"))?,
            discrimination: discrimination.ok_or_else(|| param_missing_error("discrimination"))?,
            block0_consensus: block0_consensus
                .ok_or_else(|| param_missing_error("block0_consensus"))?,
            slots_per_epoch: slots_per_epoch
                .ok_or_else(|| param_missing_error("slots_per_epoch"))?,
            slot_duration: slot_duration.ok_or_else(|| param_missing_error("slot_duration"))?,
            consensus_genesis_praos_active_slot_coeff: consensus_genesis_praos_active_slot_coeff
                .ok_or_else(|| param_missing_error("consensus_genesis_praos_active_slot_coeff"))?,
            linear_fees: linear_fees.ok_or_else(|| param_missing_error("linear_fees"))?,
            proposal_expiration: proposal_expiration
                .ok_or_else(|| param_missing_error("proposal_expiration"))?,
            kes_update_speed: kes_update_speed
                .ok_or_else(|| param_missing_error("kes_update_speed"))?,
            epoch_stability_depth: epoch_stability_depth
                .ok_or_else(|| param_missing_error("epoch_stability_depth"))?,
            consensus_leader_ids,
            block_content_max_size: block_content_max_size
                .ok_or_else(|| param_missing_error("block_content_max_size"))?,
            fees_go_to,
            treasury,
            treasury_parameters,
            total_reward_supply,
            reward_parameters,
            reward_constraints,
            committees,
            tx_max_expiry_epochs,
            #[cfg(feature = "evm")]
            evm_configs,
            #[cfg(feature = "evm")]
            evm_env_settings,
        })
    }

    fn into_config_params(self) -> ConfigParams {
        let BlockchainConfiguration {
            block0_date,
            discrimination,
            block0_consensus,
            linear_fees,
            proposal_expiration,
            consensus_leader_ids,
            slots_per_epoch,
            slot_duration,
            kes_update_speed,
            consensus_genesis_praos_active_slot_coeff,
            block_content_max_size,
            epoch_stability_depth,
            fees_go_to,
            treasury,
            treasury_parameters,
            total_reward_supply,
            reward_parameters,
            reward_constraints,
            committees,
            tx_max_expiry_epochs,
            #[cfg(feature = "evm")]
            evm_configs,
            #[cfg(feature = "evm")]
            evm_env_settings,
        } = self;

        let mut params = ConfigParams::new();

        params.push(ConfigParam::Block0Date(Block0Date(block0_date.0)));
        params.push(ConfigParam::Discrimination(discrimination));
        params.push(ConfigParam::ConsensusVersion(block0_consensus));
        params.push(ConfigParam::LinearFee(linear_fees.clone()));
        params.push(ConfigParam::from(slots_per_epoch));
        params.push(ConfigParam::from(slot_duration));
        params.push(ConfigParam::from(kes_update_speed));
        params.push(ConfigParam::from(consensus_genesis_praos_active_slot_coeff));
        params.push(ConfigParam::BlockContentMaxSize(
            block_content_max_size.into(),
        ));
        params.push(ConfigParam::EpochStabilityDepth(
            epoch_stability_depth.into(),
        ));
        params.push(ConfigParam::ProposalExpiration(proposal_expiration.into()));

        if let Some(fees_go_to) = fees_go_to {
            params.push(ConfigParam::from(fees_go_to));
        }

        if !crate::interfaces::linear_fee::per_certificate_fee_is_zero(
            &linear_fees.per_certificate_fees,
        ) {
            params.push(ConfigParam::PerCertificateFees(
                linear_fees.per_certificate_fees,
            ));
        }

        if !crate::interfaces::linear_fee::per_vote_certificate_fee_is_zero(
            &linear_fees.per_vote_certificate_fees,
        ) {
            params.push(ConfigParam::PerVoteCertificateFees(
                linear_fees.per_vote_certificate_fees,
            ));
        }

        if let Some(treasury) = treasury {
            params.push(ConfigParam::TreasuryAdd(treasury.into()));
        }

        if let Some(treasury_parameters) = treasury_parameters {
            params.push(ConfigParam::TreasuryParams(treasury_parameters.into()));
        }

        if let Some(total_reward_supply) = total_reward_supply {
            params.push(ConfigParam::RewardPot(total_reward_supply.into()));
        }

        if let Some(reward_parameters) = reward_parameters {
            params.push(ConfigParam::RewardParams(reward_parameters.into()));
        }

        if let Some(reward_drawing_limit_max) = reward_constraints.reward_drawing_limit_max {
            params.push(ConfigParam::RewardLimitByAbsoluteStake(
                reward_drawing_limit_max.into(),
            ));
        }

        if let Some(pool_participation_capping) = reward_constraints.pool_participation_capping {
            params.push(ConfigParam::PoolRewardParticipationCapping((
                pool_participation_capping.min,
                pool_participation_capping.max,
            )));
        }

        if let Some(tx_max_expiry_epochs) = tx_max_expiry_epochs {
            params.push(ConfigParam::TransactionMaxExpiryEpochs(
                tx_max_expiry_epochs,
            ));
        }

        #[cfg(feature = "evm")]
        if let Some(evm_configs) = evm_configs {
            params.push(ConfigParam::EvmConfiguration(evm_configs.into()));
        }

        #[cfg(feature = "evm")]
        if let Some(evm_env_settings) = evm_env_settings {
            params.push(ConfigParam::EvmEnvironment(evm_env_settings.into()));
        }

        let params = consensus_leader_ids
            .into_iter()
            .map(ConfigParam::from)
            .fold(params, |mut params, cp| {
                params.push(cp);
                params
            });

        committees
            .into_iter()
            .map(CommitteeId::from)
            .map(ConfigParam::AddCommitteeId)
            .fold(params, |mut params, cp| {
                params.push(cp);
                params
            })
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case", remote = "Discrimination")]
pub enum DiscriminationDef {
    Test,
    Production,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case", remote = "ConsensusVersion")]
pub enum ConsensusVersionDef {
    Bft,
    GenesisPraos,
}

#[cfg(test)]
mod test {
    use super::*;
    use chain_impl_mockchain::fee::{PerCertificateFee, PerVoteCertificateFee};
    use quickcheck::{Arbitrary, Gen};
    use std::num::NonZeroU64;

    impl Arbitrary for BlockchainConfiguration {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            let counter_leaders = usize::arbitrary(g) % 12;
            let counter_committee = usize::arbitrary(g) % 12;

            let mut linear_fees =
                LinearFee::new(u64::arbitrary(g), u64::arbitrary(g), u64::arbitrary(g));
            linear_fees.per_certificate_fees(PerCertificateFee::new(
                NonZeroU64::new(u64::arbitrary(g)),
                NonZeroU64::new(u64::arbitrary(g)),
                NonZeroU64::new(u64::arbitrary(g)),
            ));
            linear_fees.per_vote_certificate_fees(PerVoteCertificateFee::new(
                NonZeroU64::new(u64::arbitrary(g)),
                NonZeroU64::new(u64::arbitrary(g)),
            ));

            BlockchainConfiguration {
                block0_date: SecondsSinceUnixEpoch::arbitrary(g),
                discrimination: if bool::arbitrary(g) {
                    Discrimination::Production
                } else {
                    Discrimination::Test
                },
                block0_consensus: if bool::arbitrary(g) {
                    ConsensusVersion::Bft
                } else {
                    ConsensusVersion::GenesisPraos
                },
                linear_fees,
                proposal_expiration: Arbitrary::arbitrary(g),
                consensus_leader_ids: std::iter::repeat_with(|| Arbitrary::arbitrary(g))
                    .take(counter_leaders)
                    .collect(),
                slots_per_epoch: NumberOfSlotsPerEpoch::arbitrary(g),
                slot_duration: SlotDuration::arbitrary(g),
                kes_update_speed: KesUpdateSpeed::arbitrary(g),
                consensus_genesis_praos_active_slot_coeff: ActiveSlotCoefficient::arbitrary(g),
                block_content_max_size: Arbitrary::arbitrary(g),
                epoch_stability_depth: Arbitrary::arbitrary(g),
                fees_go_to: Arbitrary::arbitrary(g),
                treasury: Arbitrary::arbitrary(g),
                treasury_parameters: Arbitrary::arbitrary(g),
                total_reward_supply: Arbitrary::arbitrary(g),
                reward_parameters: Arbitrary::arbitrary(g),
                reward_constraints: Arbitrary::arbitrary(g),
                committees: std::iter::repeat_with(|| Arbitrary::arbitrary(g))
                    .take(counter_committee)
                    .collect(),
                tx_max_expiry_epochs: Arbitrary::arbitrary(g),
                #[cfg(feature = "evm")]
                evm_configs: Arbitrary::arbitrary(g),
                #[cfg(feature = "evm")]
                evm_env_settings: Arbitrary::arbitrary(g),
            }
        }
    }

    quickcheck! {
        fn serde_encode_decode(blockchain_configuration: BlockchainConfiguration) -> bool {
            let s = serde_yaml::to_string(&blockchain_configuration).unwrap();
            let blockchain_configuration_dec: BlockchainConfiguration = serde_yaml::from_str(&s).unwrap();

            blockchain_configuration == blockchain_configuration_dec
        }

        fn convert_from_to_config_param(blockchain_configuration: BlockchainConfiguration) -> bool {
            let cps = ConfigParams::from(blockchain_configuration.clone());
            let blockchain_configuration_dec = BlockchainConfiguration::try_from(cps).unwrap();

            blockchain_configuration == blockchain_configuration_dec
        }
    }
}
