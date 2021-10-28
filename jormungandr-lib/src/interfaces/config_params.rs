use super::{
    ActiveSlotCoefficient, BlockContentMaxSize, CommitteeIdDef, ConsensusLeaderId,
    ConsensusVersionDef, DiscriminationDef, EpochStabilityDepth, FeesGoTo, KesUpdateSpeed,
    LinearFeeDef, NumberOfSlotsPerEpoch, PoolParticipationCapping, Ratio, RewardParams,
    SlotDuration, TaxType, Value,
};
use crate::time::SecondsSinceUnixEpoch;
use chain_addr::Discrimination;
use chain_impl_mockchain::{
    chaintypes::ConsensusVersion,
    config::{Block0Date, ConfigParam as ConfigParamLib},
    fee::LinearFee,
    fragment::ConfigParams as ConfigParamsLib,
};
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigParams(pub(crate) Vec<ConfigParam>);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConfigParam {
    Block0Date(SecondsSinceUnixEpoch),
    #[serde(with = "DiscriminationDef")]
    Discrimination(Discrimination),
    #[serde(with = "ConsensusVersionDef")]
    ConsensusVersion(ConsensusVersion),
    SlotsPerEpoch(NumberOfSlotsPerEpoch),
    SlotDuration(SlotDuration),
    EpochStabilityDepth(EpochStabilityDepth),
    ConsensusGenesisPraosActiveSlotsCoeff(ActiveSlotCoefficient),
    BlockContentMaxSize(BlockContentMaxSize),
    AddBftLeader(ConsensusLeaderId),
    RemoveBftLeader(ConsensusLeaderId),
    #[serde(with = "LinearFeeDef")]
    LinearFee(LinearFee),
    // TODO implement interface
    ProposalExpiration(),
    KesUpdateSpeed(KesUpdateSpeed),
    TreasuryAdd(Value),
    TreasuryParams(TaxType),
    RewardPot(Value),
    RewardParams(RewardParams),
    // TODO implement interface
    PerCertificateFees(),
    FeesInTreasury(FeesGoTo),
    RewardLimitNone,
    RewardLimitByAbsoluteStake(Ratio),
    PoolRewardParticipationCapping(PoolParticipationCapping),
    AddCommitteeId(CommitteeIdDef),
    RemoveCommitteeId(CommitteeIdDef),
    // TODO implement interface
    PerVoteCertificateFees(),
    TransactionMaxExpiryEpochs(u8),
}

#[derive(Debug, Error)]
pub enum FromConfigParamError {
    #[error("Invalid number of slots per epoch")]
    NumberOfSlotsPerEpoch(#[from] super::block0_configuration::TryFromNumberOfSlotsPerEpochError),
    #[error("Invalid slot duration value")]
    SlotDuration(#[from] super::block0_configuration::TryFromSlotDurationError),
    #[error("Invalid FeesGoTo setting")]
    FeesGoTo(#[from] super::block0_configuration::TryFromFeesGoToError),
}

impl From<ConfigParams> for ConfigParamsLib {
    fn from(config: ConfigParams) -> Self {
        let mut res = Self::new();
        for el in config.0 {
            res.push(el.into());
        }
        res
    }
}

impl From<ConfigParam> for ConfigParamLib {
    fn from(config: ConfigParam) -> Self {
        match config {
            ConfigParam::Block0Date(val) => Self::Block0Date(Block0Date(val.0)),
            ConfigParam::Discrimination(val) => Self::Discrimination(val),
            ConfigParam::ConsensusVersion(val) => Self::ConsensusVersion(val),
            ConfigParam::SlotsPerEpoch(val) => Self::SlotsPerEpoch(val.into()),
            ConfigParam::SlotDuration(val) => Self::SlotDuration(val.into()),
            ConfigParam::EpochStabilityDepth(val) => Self::EpochStabilityDepth(val.into()),
            ConfigParam::ConsensusGenesisPraosActiveSlotsCoeff(val) => {
                Self::ConsensusGenesisPraosActiveSlotsCoeff(val.0)
            }
            ConfigParam::BlockContentMaxSize(val) => Self::BlockContentMaxSize(val.into()),
            ConfigParam::AddBftLeader(val) => Self::AddBftLeader(val.into()),
            ConfigParam::RemoveBftLeader(val) => Self::RemoveBftLeader(val.into()),
            ConfigParam::LinearFee(val) => Self::LinearFee(val),
            // TODO implement
            ConfigParam::ProposalExpiration() => Self::ProposalExpiration(Default::default()),
            ConfigParam::KesUpdateSpeed(val) => Self::KesUpdateSpeed(val.0),
            ConfigParam::TreasuryAdd(val) => Self::TreasuryAdd(val.into()),
            ConfigParam::TreasuryParams(val) => Self::TreasuryParams(val.into()),
            ConfigParam::RewardPot(val) => Self::RewardPot(val.into()),
            ConfigParam::RewardParams(val) => Self::RewardParams(val.into()),
            // TODO implement
            ConfigParam::PerCertificateFees() => Self::PerCertificateFees(Default::default()),
            ConfigParam::FeesInTreasury(val) => Self::from(val),
            ConfigParam::RewardLimitNone => Self::RewardLimitNone,
            ConfigParam::RewardLimitByAbsoluteStake(val) => {
                Self::RewardLimitByAbsoluteStake(val.into())
            }
            ConfigParam::PoolRewardParticipationCapping(val) => {
                Self::PoolRewardParticipationCapping((val.min, val.max))
            }
            ConfigParam::AddCommitteeId(val) => Self::AddCommitteeId(val.into()),
            ConfigParam::RemoveCommitteeId(val) => Self::RemoveCommitteeId(val.into()),
            // TODO implement
            ConfigParam::PerVoteCertificateFees() => {
                Self::PerVoteCertificateFees(Default::default())
            }
            ConfigParam::TransactionMaxExpiryEpochs(val) => Self::TransactionMaxExpiryEpochs(val),
        }
    }
}

impl TryFrom<ConfigParamLib> for ConfigParam {
    type Error = FromConfigParamError;
    fn try_from(config: ConfigParamLib) -> Result<Self, Self::Error> {
        Ok(match config {
            ConfigParamLib::Block0Date(val) => Self::Block0Date(SecondsSinceUnixEpoch(val.0)),
            ConfigParamLib::Discrimination(val) => Self::Discrimination(val),
            ConfigParamLib::ConsensusVersion(val) => Self::ConsensusVersion(val),
            config @ ConfigParamLib::SlotsPerEpoch(_) => {
                Self::SlotsPerEpoch(NumberOfSlotsPerEpoch::try_from(config)?)
            }
            config @ ConfigParamLib::SlotDuration(_) => {
                Self::SlotDuration(SlotDuration::try_from(config)?)
            }
            ConfigParamLib::EpochStabilityDepth(val) => {
                Self::EpochStabilityDepth(EpochStabilityDepth::from(val))
            }
            ConfigParamLib::ConsensusGenesisPraosActiveSlotsCoeff(val) => {
                Self::ConsensusGenesisPraosActiveSlotsCoeff(ActiveSlotCoefficient(val))
            }
            ConfigParamLib::BlockContentMaxSize(val) => Self::BlockContentMaxSize(val.into()),
            ConfigParamLib::AddBftLeader(val) => Self::AddBftLeader(ConsensusLeaderId::from(val)),
            ConfigParamLib::RemoveBftLeader(val) => {
                Self::RemoveBftLeader(ConsensusLeaderId::from(val))
            }
            ConfigParamLib::LinearFee(val) => Self::LinearFee(val),
            // TODO implement
            ConfigParamLib::ProposalExpiration(_val) => Self::ProposalExpiration(),
            ConfigParamLib::KesUpdateSpeed(val) => Self::KesUpdateSpeed(KesUpdateSpeed(val)),
            ConfigParamLib::TreasuryAdd(val) => Self::TreasuryAdd(Value::from(val)),
            ConfigParamLib::TreasuryParams(val) => Self::TreasuryParams(TaxType::from(val)),
            ConfigParamLib::RewardPot(val) => Self::RewardPot(Value::from(val)),
            ConfigParamLib::RewardParams(val) => Self::RewardParams(RewardParams::from(val)),
            // TODO implement
            ConfigParamLib::PerCertificateFees(_val) => Self::PerCertificateFees(),
            config @ ConfigParamLib::FeesInTreasury(_) => {
                Self::FeesInTreasury(FeesGoTo::try_from(config)?)
            }
            ConfigParamLib::RewardLimitNone => Self::RewardLimitNone,
            ConfigParamLib::RewardLimitByAbsoluteStake(val) => {
                Self::RewardLimitByAbsoluteStake(Ratio::from(val))
            }
            ConfigParamLib::PoolRewardParticipationCapping((min, max)) => {
                Self::PoolRewardParticipationCapping(PoolParticipationCapping { min, max })
            }
            ConfigParamLib::AddCommitteeId(val) => Self::AddCommitteeId(CommitteeIdDef::from(val)),
            ConfigParamLib::RemoveCommitteeId(val) => {
                Self::RemoveCommitteeId(CommitteeIdDef::from(val))
            }
            // TODO implement
            ConfigParamLib::PerVoteCertificateFees(_val) => Self::PerVoteCertificateFees(),
            ConfigParamLib::TransactionMaxExpiryEpochs(val) => {
                Self::TransactionMaxExpiryEpochs(val)
            }
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use quickcheck::{Arbitrary, Gen};

    impl Arbitrary for ConfigParam {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            let config = ConfigParamLib::arbitrary(g);
            Self::try_from(config).unwrap()
        }
    }

    quickcheck! {
        fn serde_encode_decode(config: ConfigParam) -> bool {
            let s = serde_yaml::to_string(&config).unwrap();
            let config_dec: ConfigParam = serde_yaml::from_str(&s).unwrap();

            config == config_dec
        }
    }
}
