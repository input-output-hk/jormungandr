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
            ConfigParam::SlotsPerEpoch(val) => Self::SlotsPerEpoch(val.0),
            ConfigParam::SlotDuration(val) => Self::SlotDuration(val.0),
            ConfigParam::EpochStabilityDepth(val) => Self::EpochStabilityDepth(val.0),
            ConfigParam::ConsensusGenesisPraosActiveSlotsCoeff(val) => {
                Self::ConsensusGenesisPraosActiveSlotsCoeff(val.0)
            }
            ConfigParam::BlockContentMaxSize(val) => Self::BlockContentMaxSize(val.into()),
            ConfigParam::AddBftLeader(val) => Self::AddBftLeader(val.0),
            ConfigParam::RemoveBftLeader(val) => Self::RemoveBftLeader(val.0),
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
            ConfigParam::FeesInTreasury(val) => Self::FeesInTreasury(val.into()),
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

impl From<ConfigParamLib> for ConfigParam {
    fn from(config: ConfigParamLib) -> Self {
        match config {
            ConfigParamLib::Block0Date(val) => Self::Block0Date(SecondsSinceUnixEpoch(val.0)),
            ConfigParamLib::Discrimination(val) => Self::Discrimination(val),
            ConfigParamLib::ConsensusVersion(val) => Self::ConsensusVersion(val),
            ConfigParamLib::SlotsPerEpoch(val) => Self::SlotsPerEpoch(NumberOfSlotsPerEpoch(val)),
            ConfigParamLib::SlotDuration(val) => Self::SlotDuration(SlotDuration(val)),
            ConfigParamLib::EpochStabilityDepth(val) => {
                Self::EpochStabilityDepth(EpochStabilityDepth(val))
            }
            ConfigParamLib::ConsensusGenesisPraosActiveSlotsCoeff(val) => {
                Self::ConsensusGenesisPraosActiveSlotsCoeff(ActiveSlotCoefficient(val))
            }
            ConfigParamLib::BlockContentMaxSize(val) => Self::BlockContentMaxSize(val.into()),
            ConfigParamLib::AddBftLeader(val) => Self::AddBftLeader(ConsensusLeaderId(val)),
            ConfigParamLib::RemoveBftLeader(val) => Self::RemoveBftLeader(ConsensusLeaderId(val)),
            ConfigParamLib::LinearFee(val) => Self::LinearFee(val),
            // TODO implement
            ConfigParamLib::ProposalExpiration(_val) => Self::ProposalExpiration(),
            ConfigParamLib::KesUpdateSpeed(val) => Self::KesUpdateSpeed(KesUpdateSpeed(val)),
            ConfigParamLib::TreasuryAdd(val) => Self::TreasuryAdd(Value(val)),
            ConfigParamLib::TreasuryParams(val) => Self::TreasuryParams(TaxType::from(val)),
            ConfigParamLib::RewardPot(val) => Self::RewardPot(Value(val)),
            ConfigParamLib::RewardParams(val) => Self::RewardParams(RewardParams::from(val)),
            // TODO implement
            ConfigParamLib::PerCertificateFees(_val) => Self::PerCertificateFees(),
            ConfigParamLib::FeesInTreasury(val) => Self::FeesInTreasury(FeesGoTo::from(val)),
            ConfigParamLib::RewardLimitNone => Self::RewardLimitNone,
            ConfigParamLib::RewardLimitByAbsoluteStake(val) => {
                Self::RewardLimitByAbsoluteStake(Ratio(val))
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
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use quickcheck::{Arbitrary, Gen};

    impl Arbitrary for ConfigParam {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            let config = ConfigParamLib::arbitrary(g);
            Self::from(config)
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
