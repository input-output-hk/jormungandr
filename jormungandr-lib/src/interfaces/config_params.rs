use super::{
    ActiveSlotCoefficient, BlockContentMaxSize, CommitteeIdDef, ConsensusLeaderId,
    ConsensusVersionDef, DiscriminationDef, EpochStabilityDepth, FeesGoTo, KesUpdateSpeed,
    LinearFeeDef, NumberOfSlotsPerEpoch, PoolParticipationCapping, Ratio, RewardParams,
    SlotDuration, TaxType, Value,
};
use crate::time::SecondsSinceUnixEpoch;
use chain_addr::Discrimination;
use chain_impl_mockchain::{
    chaintypes::ConsensusVersion, config::ConfigParam as ConfigParamLib, fee::LinearFee,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
enum ConfigParam {
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
