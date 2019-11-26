use crate::{
    interfaces::{
        ActiveSlotCoefficient, BFTSlotsRatio, ConsensusLeaderId, KESUpdateSpeed, LinearFeeDef,
        NumberOfSlotsPerEpoch, SlotDuration,
    },
    time::SecondsSinceUnixEpoch,
};
use chain_addr::Discrimination;
use chain_impl_mockchain::{
    block::ConsensusVersion,
    config::{Block0Date, ConfigParam},
    fee::LinearFee,
    fragment::config::ConfigParams,
    value,
};
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;

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
    pub kes_update_speed: KESUpdateSpeed,

    /// the active slot coefficient to determine the minimal stake
    /// in order to participate to the consensus.
    ///
    /// default value is 0.1
    #[serde(default)]
    pub consensus_genesis_praos_active_slot_coeff: ActiveSlotCoefficient,

    /// allow BFT and Genesis Praos to live together by allocating some
    /// slots to the `consensus_leader_ids` (the BFT Leaders).
    ///
    /// default value is 0.22.
    #[serde(default)]
    pub bft_slots_ratio: BFTSlotsRatio,

    /// TODO: need some love
    /// this value is left for compatibility only but should be removed or
    /// replaced by something more meaningful: max block size (in bytes)
    #[serde(default)]
    pub max_number_of_transactions_per_block: Option<u32>,

    /// TODO: need some love
    /// this value is left for compatibility only be should be removed
    /// or replaced by something more meaningful or merged with
    /// `slots_per_epoch`.
    #[serde(default)]
    pub epoch_stability_depth: Option<u32>,

    /// Set the default value in the treasury. if omitted then the treasury starts with the value of 0
    #[serde(default)]
    pub treasury: Option<u64>,

    /// Set the value of the reward pot. if omitted then the reward pot is empty
    #[serde(default)]
    pub rewards: Option<u64>,
}

impl From<BlockchainConfiguration> for ConfigParams {
    fn from(blockchain_configuration: BlockchainConfiguration) -> Self {
        blockchain_configuration.into_config_params()
    }
}

type StaticStr = &'static str;

custom_error! {pub FromConfigParamsError
    InitConfigParamMissing { name: StaticStr } = "initial message misses parameter {name}",
    InitConfigParamDuplicate { name: StaticStr } = "initial message contains duplicate parameter {name}",
    NumberOfSlotsPerEpoch { source: super::number_of_slots_per_epoch::TryFromNumberOfSlotsPerEpochError } = "Invalid number of slots per epoch",
    SlotDuration { source: super::slots_duration::TryFromSlotDurationError } = "Invalid slot duration value",
    ConsensusLeaderId { source: super::leader_id::TryFromConsensusLeaderIdError } = "Invalid consensus leader id",
    ActiveSlotCoefficient { source: super::active_slot_coefficient::TryFromActiveSlotCoefficientError } = "Invalid active slot coefficient value",
    BFTSlotsRatio { source: super::bft_slots_ratio::TryFromBFTSlotsRatioError } = "Invalid BFT Slot ratio",
    KESUpdateSpeed { source: super::kes_update_speed::TryFromKESUpdateSpeedError } = "Invalid KES Update speed value",
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
            consensus_leader_ids: Vec::default(),
            slots_per_epoch: NumberOfSlotsPerEpoch::default(),
            slot_duration: SlotDuration::default(),
            kes_update_speed: KESUpdateSpeed::default(),
            consensus_genesis_praos_active_slot_coeff: ActiveSlotCoefficient::default(),
            bft_slots_ratio: BFTSlotsRatio::default(),
            max_number_of_transactions_per_block: None,
            epoch_stability_depth: None,
            treasury: None,
            rewards: None,
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
        let mut max_number_of_transactions_per_block = None;
        let mut bft_slots_ratio = None;
        let mut linear_fees = None;
        let mut kes_update_speed = None;
        let mut treasury = None;
        let mut rewards = None;
        let mut per_certificate_fees = None;

        for param in params.iter().cloned() {
            match param {
                ConfigParam::Block0Date(param) => block0_date
                    .replace(SecondsSinceUnixEpoch::from(SecondsSinceUnixEpoch(param.0)))
                    .map(|_| "block0_date"),
                ConfigParam::ConsensusVersion(param) => {
                    block0_consensus.replace(param).map(|_| "block0_consensus")
                }
                ConfigParam::Discrimination(param) => {
                    discrimination.replace(param).map(|_| "discrimination")
                }
                cp @ ConfigParam::SlotsPerEpoch(_) => slots_per_epoch
                    .replace(NumberOfSlotsPerEpoch::try_from(cp)?)
                    .map(|_| "slots_per_epoch"),
                cp @ ConfigParam::SlotDuration(_) => slot_duration
                    .replace(SlotDuration::try_from(cp)?)
                    .map(|_| "slot_duration"),
                cp @ ConfigParam::AddBftLeader(_) => {
                    consensus_leader_ids.push(ConsensusLeaderId::try_from(cp)?);
                    None
                }
                cp @ ConfigParam::ConsensusGenesisPraosActiveSlotsCoeff(_) => {
                    consensus_genesis_praos_active_slot_coeff
                        .replace(ActiveSlotCoefficient::try_from(cp)?)
                        .map(|_| "consensus_genesis_praos_active_slot_coeff")
                }
                cp @ ConfigParam::BftSlotsRatio(_) => bft_slots_ratio
                    .replace(BFTSlotsRatio::try_from(cp)?)
                    .map(|_| "bft_slots_ratio"),
                ConfigParam::LinearFee(param) => linear_fees.replace(param).map(|_| "linear_fees"),
                cp @ ConfigParam::KESUpdateSpeed(_) => kes_update_speed
                    .replace(KESUpdateSpeed::try_from(cp)?)
                    .map(|_| "kes_update_speed"),

                ConfigParam::RemoveBftLeader(_) => {
                    panic!("block 0 attempts to remove a BFT leader")
                }
                ConfigParam::ProposalExpiration(_param) => unimplemented!(),
                ConfigParam::MaxNumberOfTransactionsPerBlock(param) => {
                    max_number_of_transactions_per_block
                        .replace(param)
                        .map(|_| "max_number_of_transactions_per_block")
                }
                ConfigParam::EpochStabilityDepth(param) => epoch_stability_depth
                    .replace(param)
                    .map(|_| "epoch_stability_depth"),
                ConfigParam::TreasuryAdd(param) => treasury.replace(param.0).map(|_| "treasury"),
                ConfigParam::TreasuryParams(_) => unimplemented!(),
                ConfigParam::RewardPot(param) => rewards.replace(param.0).map(|_| "reward-pot"),
                ConfigParam::RewardParams(_) => unimplemented!(),
                ConfigParam::PerCertificateFees(param) => per_certificate_fees
                    .replace(param)
                    .map(|_| "per_certificate_fees"),
            }
            .map(|name| Err(FromConfigParamsError::InitConfigParamDuplicate { name }))
            .unwrap_or(Ok(()))?;
        }

        if let Some(linear_fees) = &mut linear_fees {
            if let Some(per_certificate_fees) = per_certificate_fees {
                linear_fees.per_certificate_fees(per_certificate_fees);
            }
        }

        Ok(BlockchainConfiguration {
            block0_date: block0_date.ok_or(param_missing_error("block0_date"))?,
            discrimination: discrimination.ok_or(param_missing_error("discrimination"))?,
            block0_consensus: block0_consensus.ok_or(param_missing_error("block0_consensus"))?,
            slots_per_epoch: slots_per_epoch.ok_or(param_missing_error("slots_per_epoch"))?,
            slot_duration: slot_duration.ok_or(param_missing_error("slot_duration"))?,
            consensus_genesis_praos_active_slot_coeff: consensus_genesis_praos_active_slot_coeff
                .ok_or(param_missing_error(
                    "consensus_genesis_praos_active_slot_coeff",
                ))?,
            bft_slots_ratio: bft_slots_ratio.ok_or(param_missing_error("bft_slots_ratio"))?,
            linear_fees: linear_fees.ok_or(param_missing_error("linear_fees"))?,
            kes_update_speed: kes_update_speed.ok_or(param_missing_error("kes_update_speed"))?,
            epoch_stability_depth,
            consensus_leader_ids,
            max_number_of_transactions_per_block,
            treasury,
            rewards,
        })
    }

    fn into_config_params(self) -> ConfigParams {
        let BlockchainConfiguration {
            block0_date,
            discrimination,
            block0_consensus,
            linear_fees,
            consensus_leader_ids,
            slots_per_epoch,
            slot_duration,
            kes_update_speed,
            consensus_genesis_praos_active_slot_coeff,
            bft_slots_ratio,
            max_number_of_transactions_per_block,
            epoch_stability_depth,
            treasury,
            rewards,
        } = self;

        let mut params = ConfigParams::new();

        params.push(ConfigParam::Block0Date(Block0Date(block0_date.0)));
        params.push(ConfigParam::Discrimination(discrimination));
        params.push(ConfigParam::ConsensusVersion(block0_consensus));
        params.push(ConfigParam::LinearFee(linear_fees));
        params.push(ConfigParam::from(slots_per_epoch));
        params.push(ConfigParam::from(slot_duration));
        params.push(ConfigParam::from(kes_update_speed));
        params.push(ConfigParam::from(consensus_genesis_praos_active_slot_coeff));
        params.push(ConfigParam::from(bft_slots_ratio));

        if let Some(per_certificate_fees) = linear_fees.per_certificate_fees {
            params.push(ConfigParam::PerCertificateFees(per_certificate_fees))
        }

        if let Some(max_number_of_transactions_per_block) = max_number_of_transactions_per_block {
            params.push(ConfigParam::MaxNumberOfTransactionsPerBlock(
                max_number_of_transactions_per_block,
            ));
        }

        if let Some(epoch_stability_depth) = epoch_stability_depth {
            params.push(ConfigParam::EpochStabilityDepth(epoch_stability_depth));
        }

        if let Some(treasury) = treasury {
            params.push(ConfigParam::TreasuryAdd(value::Value(treasury)));
        }
        if let Some(rewards) = rewards {
            params.push(ConfigParam::RewardPot(value::Value(rewards)));
        }

        consensus_leader_ids
            .into_iter()
            .map(ConfigParam::from)
            .fold(params, |mut params, cp| {
                params.push(cp);
                params
            })
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case", remote = "Discrimination")]
enum DiscriminationDef {
    Test,
    Production,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case", remote = "ConsensusVersion")]
enum ConsensusVersionDef {
    Bft,
    GenesisPraos,
}

#[cfg(test)]
mod test {
    use super::*;
    use quickcheck::{Arbitrary, Gen};

    impl Arbitrary for BlockchainConfiguration {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
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
                linear_fees: LinearFee::new(
                    u64::arbitrary(g),
                    u64::arbitrary(g),
                    u64::arbitrary(g),
                ),
                consensus_leader_ids: Arbitrary::arbitrary(g),
                slots_per_epoch: NumberOfSlotsPerEpoch::arbitrary(g),
                slot_duration: SlotDuration::arbitrary(g),
                kes_update_speed: KESUpdateSpeed::arbitrary(g),
                consensus_genesis_praos_active_slot_coeff: ActiveSlotCoefficient::arbitrary(g),
                bft_slots_ratio: BFTSlotsRatio::arbitrary(g),
                max_number_of_transactions_per_block: Arbitrary::arbitrary(g),
                epoch_stability_depth: Arbitrary::arbitrary(g),
                treasury: Arbitrary::arbitrary(g),
                rewards: Arbitrary::arbitrary(g),
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
