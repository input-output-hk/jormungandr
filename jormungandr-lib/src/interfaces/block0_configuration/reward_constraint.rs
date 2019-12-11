use crate::interfaces::Ratio;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct RewardConstraints {
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reward_drawing_limit_max: Option<Ratio>,
}

impl RewardConstraints {
    pub fn is_none(&self) -> bool {
        self.reward_drawing_limit_max.is_none()
    }

    pub fn set_reward_drawing_limit_max(&mut self, limit: Option<Ratio>) {
        self.reward_drawing_limit_max = limit
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use quickcheck::{Arbitrary, Gen};

    impl Arbitrary for RewardConstraints {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            Self {
                reward_drawing_limit_max: Arbitrary::arbitrary(g),
            }
        }
    }
}
