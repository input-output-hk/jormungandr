use crate::interfaces::Ratio;
use serde::{Deserialize, Serialize};
use std::num::NonZeroU32;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PoolParticipationCapping {
    pub min: NonZeroU32,
    pub max: NonZeroU32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct RewardConstraints {
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reward_drawing_limit_max: Option<Ratio>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pool_participation_capping: Option<PoolParticipationCapping>,
}

impl RewardConstraints {
    pub fn is_none(&self) -> bool {
        self.reward_drawing_limit_max.is_none() && self.pool_participation_capping.is_none()
    }

    pub fn set_reward_drawing_limit_max(&mut self, limit: Option<Ratio>) {
        self.reward_drawing_limit_max = limit
    }

    pub fn set_pool_participation_capping(&mut self, setting: Option<(NonZeroU32, NonZeroU32)>) {
        let setting = setting.map(|(min, max)| PoolParticipationCapping { min, max });

        self.pool_participation_capping = setting;
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use quickcheck::{Arbitrary, Gen};

    impl Arbitrary for PoolParticipationCapping {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            Self {
                min: NonZeroU32::new(Arbitrary::arbitrary(g))
                    .unwrap_or(unsafe { NonZeroU32::new_unchecked(1) }),
                max: NonZeroU32::new(Arbitrary::arbitrary(g))
                    .unwrap_or(unsafe { NonZeroU32::new_unchecked(1) }),
            }
        }
    }

    impl Arbitrary for RewardConstraints {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            Self {
                reward_drawing_limit_max: Arbitrary::arbitrary(g),
                pool_participation_capping: Arbitrary::arbitrary(g),
            }
        }
    }
}
