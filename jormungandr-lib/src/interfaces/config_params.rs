use super::{
    ActiveSlotCoefficient, BlockContentMaxSize, CommitteeIdDef, ConsensusLeaderId,
    ConsensusVersionDef, DiscriminationDef, EpochStabilityDepth, FeesGoTo, KesUpdateSpeed,
    LinearFeeDef, NumberOfSlotsPerEpoch, PerCertificateFeeDef, PerVoteCertificateFeeDef,
    PoolParticipationCapping, ProposalExpiration, Ratio, RewardParams, SlotDuration, TaxType,
    Value,
};
use crate::time::SecondsSinceUnixEpoch;
use chain_addr::Discrimination;
use chain_impl_mockchain::{
    chaintypes::ConsensusVersion,
    config::{Block0Date, ConfigParam as ConfigParamLib},
    fee::{LinearFee, PerCertificateFee, PerVoteCertificateFee},
    fragment::ConfigParams as ConfigParamsLib,
};
use serde::{Deserialize, Serialize};
use std::convert::{TryFrom, TryInto};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigParams(Vec<ConfigParam>);

impl ConfigParams {
    pub fn new(vec: Vec<ConfigParam>) -> Self {
        Self(vec)
    }
}

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
    ProposalExpiration(ProposalExpiration),
    KesUpdateSpeed(KesUpdateSpeed),
    TreasuryAdd(Value),
    TreasuryParams(TaxType),
    RewardPot(Value),
    RewardParams(RewardParams),
    #[serde(with = "PerCertificateFeeDef")]
    PerCertificateFees(PerCertificateFee),
    FeesInTreasury(FeesGoTo),
    RewardLimitNone,
    RewardLimitByAbsoluteStake(Ratio),
    PoolRewardParticipationCapping(PoolParticipationCapping),
    AddCommitteeId(CommitteeIdDef),
    RemoveCommitteeId(CommitteeIdDef),
    #[serde(with = "PerVoteCertificateFeeDef")]
    PerVoteCertificateFees(PerVoteCertificateFee),
    TransactionMaxExpiryEpochs(u8),
    #[cfg(feature = "evm")]
    EvmConfiguration(super::evm_params::EvmConfig),
    #[cfg(feature = "evm")]
    EvmEnvironment(super::evm_params::EvmEnvSettings),
}

#[derive(Debug, Error)]
pub enum FromConfigParamError {
    #[error("Invalid number of slots per epoch")]
    NumberOfSlotsPerEpoch(#[from] super::block0_configuration::TryFromNumberOfSlotsPerEpochError),
    #[error("Invalid slot duration value")]
    SlotDuration(#[from] super::block0_configuration::TryFromSlotDurationError),
    #[error("Invalid FeesGoTo setting")]
    FeesGoTo(#[from] super::block0_configuration::TryFromFeesGoToError),
    #[error("Invalid active slot coefficient value")]
    ActiveSlotCoefficient(#[from] super::block0_configuration::TryFromActiveSlotCoefficientError),
    #[error("Invalid KES Update speed value")]
    KesUpdateSpeed(#[from] super::block0_configuration::TryFromKesUpdateSpeedError),
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
            ConfigParam::ConsensusGenesisPraosActiveSlotsCoeff(val) => val.into(),
            ConfigParam::BlockContentMaxSize(val) => Self::BlockContentMaxSize(val.into()),
            ConfigParam::AddBftLeader(val) => Self::AddBftLeader(val.into()),
            ConfigParam::RemoveBftLeader(val) => Self::RemoveBftLeader(val.into()),
            ConfigParam::LinearFee(val) => Self::LinearFee(val),
            ConfigParam::ProposalExpiration(val) => Self::ProposalExpiration(val.into()),
            ConfigParam::KesUpdateSpeed(val) => val.into(),
            ConfigParam::TreasuryAdd(val) => Self::TreasuryAdd(val.into()),
            ConfigParam::TreasuryParams(val) => Self::TreasuryParams(val.into()),
            ConfigParam::RewardPot(val) => Self::RewardPot(val.into()),
            ConfigParam::RewardParams(val) => Self::RewardParams(val.into()),
            ConfigParam::PerCertificateFees(val) => Self::PerCertificateFees(val),
            ConfigParam::FeesInTreasury(val) => val.into(),
            ConfigParam::RewardLimitNone => Self::RewardLimitNone,
            ConfigParam::RewardLimitByAbsoluteStake(val) => {
                Self::RewardLimitByAbsoluteStake(val.into())
            }
            ConfigParam::PoolRewardParticipationCapping(val) => {
                Self::PoolRewardParticipationCapping((val.min, val.max))
            }
            ConfigParam::AddCommitteeId(val) => Self::AddCommitteeId(val.into()),
            ConfigParam::RemoveCommitteeId(val) => Self::RemoveCommitteeId(val.into()),
            ConfigParam::PerVoteCertificateFees(val) => Self::PerVoteCertificateFees(val),
            ConfigParam::TransactionMaxExpiryEpochs(val) => Self::TransactionMaxExpiryEpochs(val),
            #[cfg(feature = "evm")]
            ConfigParam::EvmConfiguration(val) => Self::EvmConfiguration(val.into()),
            #[cfg(feature = "evm")]
            ConfigParam::EvmEnvironment(val) => Self::EvmEnvironment(val.into()),
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
            config @ ConfigParamLib::SlotsPerEpoch(_) => Self::SlotsPerEpoch(config.try_into()?),
            config @ ConfigParamLib::SlotDuration(_) => Self::SlotDuration(config.try_into()?),
            ConfigParamLib::EpochStabilityDepth(val) => Self::EpochStabilityDepth(val.into()),
            config @ ConfigParamLib::ConsensusGenesisPraosActiveSlotsCoeff(_) => {
                Self::ConsensusGenesisPraosActiveSlotsCoeff(config.try_into()?)
            }
            ConfigParamLib::BlockContentMaxSize(val) => Self::BlockContentMaxSize(val.into()),
            ConfigParamLib::AddBftLeader(val) => Self::AddBftLeader(val.into()),
            ConfigParamLib::RemoveBftLeader(val) => Self::RemoveBftLeader(val.into()),
            ConfigParamLib::LinearFee(val) => Self::LinearFee(val),
            ConfigParamLib::ProposalExpiration(val) => Self::ProposalExpiration(val.into()),
            config @ ConfigParamLib::KesUpdateSpeed(_) => Self::KesUpdateSpeed(config.try_into()?),
            ConfigParamLib::TreasuryAdd(val) => Self::TreasuryAdd(val.into()),
            ConfigParamLib::TreasuryParams(val) => Self::TreasuryParams(val.into()),
            ConfigParamLib::RewardPot(val) => Self::RewardPot(val.into()),
            ConfigParamLib::RewardParams(val) => Self::RewardParams(val.into()),
            ConfigParamLib::PerCertificateFees(val) => Self::PerCertificateFees(val),
            config @ ConfigParamLib::FeesInTreasury(_) => Self::FeesInTreasury(config.try_into()?),
            ConfigParamLib::RewardLimitNone => Self::RewardLimitNone,
            ConfigParamLib::RewardLimitByAbsoluteStake(val) => {
                Self::RewardLimitByAbsoluteStake(val.into())
            }
            ConfigParamLib::PoolRewardParticipationCapping((min, max)) => {
                Self::PoolRewardParticipationCapping(PoolParticipationCapping { min, max })
            }
            ConfigParamLib::AddCommitteeId(val) => Self::AddCommitteeId(val.into()),
            ConfigParamLib::RemoveCommitteeId(val) => Self::RemoveCommitteeId(val.into()),
            ConfigParamLib::PerVoteCertificateFees(val) => Self::PerVoteCertificateFees(val),
            ConfigParamLib::TransactionMaxExpiryEpochs(val) => {
                Self::TransactionMaxExpiryEpochs(val)
            }
            #[cfg(feature = "evm")]
            ConfigParamLib::EvmConfiguration(val) => Self::EvmConfiguration(val.into()),
            #[cfg(feature = "evm")]
            ConfigParamLib::EvmEnvironment(val) => Self::EvmEnvironment(val.into()),
        })
    }
}

pub fn config_params_documented_example() -> String {
    include_str!("CONFIG_PARAMS_DOCUMENTED_EXAMPLE.yaml").to_string()
}

#[cfg(test)]
mod test {
    use super::*;
    use quickcheck::{Arbitrary, Gen};

    impl Arbitrary for ConfigParam {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            match u8::arbitrary(g) % 30 {
                0 => Self::Block0Date(Arbitrary::arbitrary(g)),
                1 => Self::Discrimination(Arbitrary::arbitrary(g)),
                2 => Self::ConsensusVersion(Arbitrary::arbitrary(g)),
                3 => Self::SlotsPerEpoch(Arbitrary::arbitrary(g)),
                4 => Self::SlotDuration(Arbitrary::arbitrary(g)),
                5 => Self::ConsensusGenesisPraosActiveSlotsCoeff(Arbitrary::arbitrary(g)),
                6 => Self::BlockContentMaxSize(Arbitrary::arbitrary(g)),
                7 => Self::AddBftLeader(Arbitrary::arbitrary(g)),
                8 => Self::RemoveBftLeader(Arbitrary::arbitrary(g)),
                9 => Self::LinearFee(Arbitrary::arbitrary(g)),
                10 => Self::ProposalExpiration(Arbitrary::arbitrary(g)),
                11 => Self::TreasuryAdd(Arbitrary::arbitrary(g)),
                12 => Self::RewardPot(Arbitrary::arbitrary(g)),
                13 => Self::RewardParams(Arbitrary::arbitrary(g)),
                14 => Self::PerCertificateFees(Arbitrary::arbitrary(g)),
                15 => Self::FeesInTreasury(Arbitrary::arbitrary(g)),
                16 => Self::AddCommitteeId(Arbitrary::arbitrary(g)),
                17 => Self::RemoveCommitteeId(Arbitrary::arbitrary(g)),
                18 => Self::PerVoteCertificateFees(Arbitrary::arbitrary(g)),
                19 => Self::RewardPot(Arbitrary::arbitrary(g)),
                20 => Self::RewardParams(Arbitrary::arbitrary(g)),
                21 => Self::RewardParams(Arbitrary::arbitrary(g)),
                22 => Self::FeesInTreasury(Arbitrary::arbitrary(g)),
                23 => Self::RewardLimitNone,
                24 => Self::RewardLimitByAbsoluteStake(Arbitrary::arbitrary(g)),
                25 => Self::PoolRewardParticipationCapping(Arbitrary::arbitrary(g)),
                26 => Self::AddCommitteeId(Arbitrary::arbitrary(g)),
                27 => Self::RemoveCommitteeId(Arbitrary::arbitrary(g)),
                28 => Self::PerCertificateFees(Arbitrary::arbitrary(g)),
                29 => Self::TransactionMaxExpiryEpochs(Arbitrary::arbitrary(g)),
                _ => unreachable!(),
            }
        }
    }

    #[test]
    fn documented_example_decodes() {
        let _: ConfigParams = serde_yaml::from_str(&config_params_documented_example()).unwrap();
    }

    quickcheck! {
        fn serde_encode_decode(config: ConfigParam) -> bool {
            let s = serde_yaml::to_string(&config).unwrap();
            let config_dec: ConfigParam = serde_yaml::from_str(&s).unwrap();

            config == config_dec
        }
    }
}
