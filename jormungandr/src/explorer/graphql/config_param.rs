use super::{scalars::Value, BftLeader, Ratio, TaxType};
use async_graphql::{Enum, SimpleObject, Union};
use chain_addr::Discrimination as DiscriminationLib;
use chain_impl_mockchain::{
    chaintypes::ConsensusType as ContainerTypeLib,
    config::{
        Block0Date as Block0DateLib, ConfigParam as ConfigParamLib, RewardParams as RewardParamsLib,
    },
    fee::{
        LinearFee as LinearFeeLib, PerCertificateFee as PerCertificateFeeLib,
        PerVoteCertificateFee as PerVoteCertificateFeeLib,
    },
    fragment::ConfigParams as ConfigParamsLib,
    key::BftLeaderId,
    milli::Milli as MilliLib,
    rewards::{Ratio as RatioLib, TaxType as TaxTypeLib},
    value::Value as ValueLib,
    vote::CommitteeId,
};
use std::num::{NonZeroU32, NonZeroU64};

#[derive(SimpleObject)]
pub struct Block0Date {
    block0_date: u64,
}

impl From<&Block0DateLib> for Block0Date {
    fn from(v: &Block0DateLib) -> Self {
        Self { block0_date: v.0 }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Enum)]
pub enum DiscriminationEnum {
    Production,
    Test,
}

#[derive(SimpleObject)]
pub struct Discrimination {
    discrimination: DiscriminationEnum,
}

impl From<&DiscriminationLib> for Discrimination {
    fn from(v: &DiscriminationLib) -> Self {
        match v {
            DiscriminationLib::Production => Self {
                discrimination: DiscriminationEnum::Production,
            },
            DiscriminationLib::Test => Self {
                discrimination: DiscriminationEnum::Test,
            },
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Enum)]
pub enum ConsensusTypeEnum {
    Bft,
    GenesisPraos,
}

#[derive(SimpleObject)]
pub struct ConsensusType {
    consensus_type: ConsensusTypeEnum,
}

impl From<&ContainerTypeLib> for ConsensusType {
    fn from(v: &ContainerTypeLib) -> Self {
        match v {
            ContainerTypeLib::Bft => Self {
                consensus_type: ConsensusTypeEnum::Bft,
            },
            ContainerTypeLib::GenesisPraos => Self {
                consensus_type: ConsensusTypeEnum::GenesisPraos,
            },
        }
    }
}

#[derive(SimpleObject)]
pub struct SlotsPerEpoch {
    slots_per_epoch: u32,
}

impl From<&u32> for SlotsPerEpoch {
    fn from(v: &u32) -> Self {
        Self {
            slots_per_epoch: *v,
        }
    }
}

#[derive(SimpleObject)]
pub struct SlotDuration {
    slot_duration: u8,
}

impl From<&u8> for SlotDuration {
    fn from(v: &u8) -> Self {
        Self { slot_duration: *v }
    }
}

#[derive(SimpleObject)]
pub struct EpochStabilityDepth {
    epoch_stability_depth: u32,
}

impl From<&u32> for EpochStabilityDepth {
    fn from(v: &u32) -> Self {
        Self {
            epoch_stability_depth: *v,
        }
    }
}

#[derive(SimpleObject)]
pub struct Milli {
    milli: u64,
}

impl From<&MilliLib> for Milli {
    fn from(v: &MilliLib) -> Self {
        Self {
            milli: (*v).to_millis(),
        }
    }
}

#[derive(SimpleObject)]
pub struct BlockContentMaxSize {
    block_content_max_size: u32,
}

impl From<&u32> for BlockContentMaxSize {
    fn from(v: &u32) -> Self {
        Self {
            block_content_max_size: *v,
        }
    }
}

#[derive(SimpleObject)]
pub struct AddBftLeader {
    add_bft_leader: BftLeader,
}

impl From<&BftLeaderId> for AddBftLeader {
    fn from(v: &BftLeaderId) -> Self {
        Self {
            add_bft_leader: v.clone().into(),
        }
    }
}

#[derive(SimpleObject)]
pub struct RemoveBftLeader {
    remove_bft_leader: BftLeader,
}

impl From<&BftLeaderId> for RemoveBftLeader {
    fn from(v: &BftLeaderId) -> Self {
        Self {
            remove_bft_leader: v.clone().into(),
        }
    }
}

#[derive(SimpleObject)]
pub struct LinearFee {
    constant: u64,
    coefficient: u64,
    certificate: u64,
    per_certificate_fees: PerCertificateFee,
    per_vote_certificate_fees: PerVoteCertificateFee,
}

#[derive(SimpleObject)]
pub struct PerCertificateFee {
    certificate_pool_registration: Option<NonZeroU64>,
    certificate_stake_delegation: Option<NonZeroU64>,
    certificate_owner_stake_delegation: Option<NonZeroU64>,
}

impl From<&PerCertificateFeeLib> for PerCertificateFee {
    fn from(v: &PerCertificateFeeLib) -> Self {
        Self {
            certificate_pool_registration: v.certificate_pool_registration,
            certificate_stake_delegation: v.certificate_owner_stake_delegation,
            certificate_owner_stake_delegation: v.certificate_owner_stake_delegation,
        }
    }
}

#[derive(SimpleObject)]
pub struct PerVoteCertificateFee {
    certificate_vote_plan: Option<NonZeroU64>,
    certificate_vote_cast: Option<NonZeroU64>,
}

impl From<&PerVoteCertificateFeeLib> for PerVoteCertificateFee {
    fn from(v: &PerVoteCertificateFeeLib) -> Self {
        Self {
            certificate_vote_plan: v.certificate_vote_plan,
            certificate_vote_cast: v.certificate_vote_cast,
        }
    }
}

impl From<&LinearFeeLib> for LinearFee {
    fn from(v: &LinearFeeLib) -> Self {
        Self {
            constant: v.constant,
            coefficient: v.coefficient,
            certificate: v.certificate,
            per_certificate_fees: PerCertificateFee {
                certificate_pool_registration: v.per_certificate_fees.certificate_pool_registration,
                certificate_stake_delegation: v.per_certificate_fees.certificate_stake_delegation,
                certificate_owner_stake_delegation: v
                    .per_certificate_fees
                    .certificate_owner_stake_delegation,
            },
            per_vote_certificate_fees: PerVoteCertificateFee {
                certificate_vote_plan: v.per_vote_certificate_fees.certificate_vote_plan,
                certificate_vote_cast: v.per_vote_certificate_fees.certificate_vote_cast,
            },
        }
    }
}

#[derive(SimpleObject)]
pub struct ProposalExpiration {
    proposal_expiration: u32,
}

impl From<&u32> for ProposalExpiration {
    fn from(v: &u32) -> Self {
        Self {
            proposal_expiration: *v,
        }
    }
}

#[derive(SimpleObject)]
pub struct KesUpdateSpeed {
    kes_update_speed: u32,
}

impl From<&u32> for KesUpdateSpeed {
    fn from(v: &u32) -> Self {
        Self {
            kes_update_speed: *v,
        }
    }
}

#[derive(SimpleObject)]
pub struct TreasuryAdd {
    treasury_add: Value,
}

impl From<&ValueLib> for TreasuryAdd {
    fn from(v: &ValueLib) -> Self {
        Self {
            treasury_add: Value(*v),
        }
    }
}

#[derive(SimpleObject)]
pub struct TreasuryParams {
    treasury_params: TaxType,
}

impl From<&TaxTypeLib> for TreasuryParams {
    fn from(v: &TaxTypeLib) -> Self {
        Self {
            treasury_params: TaxType(*v),
        }
    }
}

#[derive(SimpleObject)]
pub struct RewardPot {
    reward_pot: Value,
}

impl From<&ValueLib> for RewardPot {
    fn from(v: &ValueLib) -> Self {
        Self {
            reward_pot: Value(*v),
        }
    }
}

#[derive(SimpleObject)]
pub struct LinearRewardParams {
    constant: u64,
    ratio: Ratio,
    epoch_start: u32,
    epoch_rate: NonZeroU32,
}

#[derive(SimpleObject)]
pub struct HalvingRewardParams {
    constant: u64,
    ratio: Ratio,
    epoch_start: u32,
    epoch_rate: NonZeroU32,
}

#[derive(Union)]
pub enum RewardParamsUnion {
    Linear(LinearRewardParams),
    Halving(HalvingRewardParams),
}

#[derive(SimpleObject)]
pub struct RewardParams {
    reward_params: RewardParamsUnion,
}

impl From<&RewardParamsLib> for RewardParams {
    fn from(v: &RewardParamsLib) -> Self {
        match v {
            RewardParamsLib::Linear {
                constant,
                ratio,
                epoch_start,
                epoch_rate,
            } => Self {
                reward_params: RewardParamsUnion::Linear(LinearRewardParams {
                    constant: *constant,
                    ratio: Ratio(*ratio),
                    epoch_start: *epoch_start,
                    epoch_rate: *epoch_rate,
                }),
            },
            RewardParamsLib::Halving {
                constant,
                ratio,
                epoch_start,
                epoch_rate,
            } => Self {
                reward_params: RewardParamsUnion::Halving(HalvingRewardParams {
                    constant: *constant,
                    ratio: Ratio(*ratio),
                    epoch_start: *epoch_start,
                    epoch_rate: *epoch_rate,
                }),
            },
        }
    }
}

#[derive(SimpleObject)]
pub struct FeesInTreasury {
    fees_in_treasury: bool,
}

impl From<&bool> for FeesInTreasury {
    fn from(v: &bool) -> Self {
        Self {
            fees_in_treasury: *v,
        }
    }
}

#[derive(SimpleObject)]
pub struct RewardLimitNone {
    reward_limit_none: bool,
}

#[derive(SimpleObject)]
pub struct RewardLimitByAbsoluteStake {
    reward_limit_by_absolute_stake: Ratio,
}

impl From<&RatioLib> for RewardLimitByAbsoluteStake {
    fn from(v: &RatioLib) -> Self {
        Self {
            reward_limit_by_absolute_stake: Ratio(*v),
        }
    }
}

#[derive(SimpleObject)]
pub struct PoolRewardParticipationCapping {
    min: NonZeroU32,
    max: NonZeroU32,
}

impl From<&(NonZeroU32, NonZeroU32)> for PoolRewardParticipationCapping {
    fn from(v: &(NonZeroU32, NonZeroU32)) -> Self {
        Self { min: v.0, max: v.1 }
    }
}

#[derive(SimpleObject)]
pub struct AddCommitteeId {
    add_committee_id: String,
}

impl From<&CommitteeId> for AddCommitteeId {
    fn from(v: &CommitteeId) -> Self {
        Self {
            add_committee_id: (*v).to_hex(),
        }
    }
}

#[derive(SimpleObject)]
pub struct RemoveCommitteeId {
    remove_committee_id: String,
}

impl From<&CommitteeId> for RemoveCommitteeId {
    fn from(v: &CommitteeId) -> Self {
        Self {
            remove_committee_id: (*v).to_hex(),
        }
    }
}

#[derive(SimpleObject)]
pub struct TransactionMaxExpiryEpochs {
    transaction_max_expiry_epochs: u8,
}

impl From<&u8> for TransactionMaxExpiryEpochs {
    fn from(v: &u8) -> Self {
        Self {
            transaction_max_expiry_epochs: *v,
        }
    }
}

#[derive(Union)]
pub enum ConfigParam {
    Block0Date(Block0Date),
    Discrimination(Discrimination),
    ConsensusVersion(ConsensusType),
    SlotsPerEpoch(SlotsPerEpoch),
    SlotDuration(SlotDuration),
    EpochStabilityDepth(EpochStabilityDepth),
    ConsensusGenesisPraosActiveSlotsCoeff(Milli),
    BlockContentMaxSize(BlockContentMaxSize),
    AddBftLeader(AddBftLeader),
    RemoveBftLeader(RemoveBftLeader),
    LinearFee(LinearFee),
    ProposalExpiration(ProposalExpiration),
    KesUpdateSpeed(KesUpdateSpeed),
    TreasuryAdd(TreasuryAdd),
    TreasuryParams(TreasuryParams),
    RewardPot(RewardPot),
    RewardParams(RewardParams),
    PerCertificateFee(PerCertificateFee),
    FeesInTreasury(FeesInTreasury),
    RewardLimitNone(RewardLimitNone),
    RewardLimitByAbsoluteStake(RewardLimitByAbsoluteStake),
    PoolRewardParticipationCapping(PoolRewardParticipationCapping),
    AddCommitteeId(AddCommitteeId),
    RemoveCommitteeId(RemoveCommitteeId),
    PerVoteCertificateFees(PerVoteCertificateFee),
    TransactionMaxExpiryEpochs(TransactionMaxExpiryEpochs),
}

#[derive(SimpleObject)]
pub struct ConfigParams {
    config_params: Vec<ConfigParam>,
}

impl From<&ConfigParamLib> for ConfigParam {
    fn from(v: &ConfigParamLib) -> Self {
        match v {
            ConfigParamLib::Block0Date(v) => Self::Block0Date(v.into()),
            ConfigParamLib::Discrimination(v) => Self::Discrimination(v.into()),
            ConfigParamLib::ConsensusVersion(v) => Self::ConsensusVersion(v.into()),
            ConfigParamLib::SlotsPerEpoch(v) => Self::SlotsPerEpoch(v.into()),
            ConfigParamLib::SlotDuration(v) => Self::SlotDuration(v.into()),
            ConfigParamLib::EpochStabilityDepth(v) => Self::EpochStabilityDepth(v.into()),
            ConfigParamLib::ConsensusGenesisPraosActiveSlotsCoeff(v) => {
                Self::ConsensusGenesisPraosActiveSlotsCoeff(v.into())
            }
            ConfigParamLib::BlockContentMaxSize(v) => Self::BlockContentMaxSize(v.into()),
            ConfigParamLib::AddBftLeader(v) => Self::AddBftLeader(v.into()),
            ConfigParamLib::RemoveBftLeader(v) => Self::RemoveBftLeader(v.into()),
            ConfigParamLib::LinearFee(v) => Self::LinearFee(v.into()),
            ConfigParamLib::ProposalExpiration(v) => Self::ProposalExpiration(v.into()),
            ConfigParamLib::KesUpdateSpeed(v) => Self::KesUpdateSpeed(v.into()),
            ConfigParamLib::TreasuryAdd(v) => Self::TreasuryAdd(v.into()),
            ConfigParamLib::TreasuryParams(v) => Self::TreasuryParams(v.into()),
            ConfigParamLib::RewardPot(v) => Self::RewardPot(v.into()),
            ConfigParamLib::RewardParams(v) => Self::RewardParams(v.into()),
            ConfigParamLib::PerCertificateFees(v) => Self::PerCertificateFee(v.into()),
            ConfigParamLib::FeesInTreasury(v) => Self::FeesInTreasury(v.into()),
            ConfigParamLib::RewardLimitNone => Self::RewardLimitNone(RewardLimitNone {
                reward_limit_none: true,
            }),
            ConfigParamLib::RewardLimitByAbsoluteStake(v) => {
                Self::RewardLimitByAbsoluteStake(v.into())
            }
            ConfigParamLib::PoolRewardParticipationCapping(v) => {
                Self::PoolRewardParticipationCapping(v.into())
            }
            ConfigParamLib::AddCommitteeId(v) => Self::AddCommitteeId(v.into()),
            ConfigParamLib::RemoveCommitteeId(v) => Self::RemoveCommitteeId(v.into()),
            ConfigParamLib::PerVoteCertificateFees(v) => Self::PerVoteCertificateFees(v.into()),
            ConfigParamLib::TransactionMaxExpiryEpochs(v) => {
                Self::TransactionMaxExpiryEpochs(v.into())
            }
            #[cfg(feature = "evm")]
            ConfigParamLib::EvmParams(_params) => todo!(),
        }
    }
}

impl From<&ConfigParamsLib> for ConfigParams {
    fn from(v: &ConfigParamsLib) -> Self {
        let mut config_params = Vec::new();
        for el in v.iter() {
            config_params.push(el.into());
        }
        Self { config_params }
    }
}
