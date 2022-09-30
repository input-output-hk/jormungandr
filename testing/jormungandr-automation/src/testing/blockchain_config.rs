use chain_impl_mockchain::rewards::{CompoundingType, Limit, Parameters};
use jormungandr_lib::interfaces::{BlockchainConfiguration, RewardParams};

pub trait BlockchainConfigurationExtension {
    fn reward_parameters(&self) -> Option<Parameters>;
}

impl BlockchainConfigurationExtension for BlockchainConfiguration {
    fn reward_parameters(&self) -> Option<Parameters> {
        let reward_param = match self.reward_parameters {
            None => return None,
            Some(r) => r,
        };

        let reward_drawing = match self.reward_constraints.reward_drawing_limit_max {
            None => Limit::None,
            Some(r) => Limit::ByStakeAbsolute(r.into()),
        };

        let pool_participation = self
            .reward_constraints
            .pool_participation_capping
            .map(|p| (p.min, p.max));

        match reward_param {
            RewardParams::Linear {
                constant,
                ratio,
                epoch_start,
                epoch_rate,
            } => Some(Parameters {
                initial_value: constant,
                compounding_ratio: ratio.into(),
                compounding_type: CompoundingType::Linear,
                epoch_rate,
                epoch_start,
                reward_drawing_limit_max: reward_drawing,
                pool_participation_capping: pool_participation,
            }),
            RewardParams::Halving {
                constant,
                ratio,
                epoch_start,
                epoch_rate,
            } => Some(Parameters {
                initial_value: constant,
                compounding_ratio: ratio.into(),
                compounding_type: CompoundingType::Halvening,
                epoch_rate,
                epoch_start,
                reward_drawing_limit_max: reward_drawing,
                pool_participation_capping: pool_participation,
            }),
        }
    }
}
