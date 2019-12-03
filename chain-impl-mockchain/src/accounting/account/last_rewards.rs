use crate::header::Epoch;
use crate::value::Value;

/// Last rewards associated with a state
///
/// It tracks the epoch where the rewards has been received,
/// and the total amount of reward for such an epoch
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LastRewards {
    pub epoch: Epoch,
    pub reward: Value,
}

impl LastRewards {
    /// Create an initial value of epoch=0 reward=0
    ///
    /// It is also safe as the "uninitialized" value, since
    /// epoch 0 doesn't have by construction any reward associated.
    pub fn default() -> Self {
        LastRewards {
            epoch: 0,
            reward: Value::zero(),
        }
    }

    /// Add some value to the last reward, if the epoch is the same, then the
    /// result is just added, however.account
    ///
    /// This should never be used with an epoch less than the last set epoch,
    /// as it would means the rewards system is rewarding something from a past state.
    pub fn add_for(&mut self, epoch: Epoch, value: Value) {
        assert!(epoch >= self.epoch);
        if self.epoch == epoch {
            self.reward = (self.reward + value).unwrap()
        } else {
            self.epoch = epoch;
            self.reward = value;
        }
    }
}
