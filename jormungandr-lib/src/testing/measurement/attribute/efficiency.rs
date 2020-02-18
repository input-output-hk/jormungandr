use crate::testing::measurement::{status::Status, thresholds::Thresholds};
use std::{cmp::Ordering, fmt};

#[derive(Clone, Debug)]
pub struct Efficiency {
    counted: u32,
    max: u32,
}

impl PartialOrd for Efficiency {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.counted.cmp(&other.counted))
    }
}

impl PartialEq for Efficiency {
    fn eq(&self, other: &Self) -> bool {
        self.counted == other.counted
    }
}

impl fmt::Display for Efficiency {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let counted = self.counted as f32;
        let max = self.max as f32;
        let percentage = (counted / max) * 100.0;
        write!(f, "{:.1} % ({}/{}).", percentage, self.counted, self.max)
    }
}

impl Efficiency {
    pub fn new(counted: u32, max: u32) -> Self {
        Self { counted, max }
    }

    pub fn against(&self, thresholds: &Thresholds<Efficiency>) -> Status {
        let green = thresholds.green_threshold();
        let yellow = thresholds.yellow_threshold();

        if *self >= green {
            return Status::Green;
        }
        if *self >= yellow {
            return Status::Yellow;
        }
        Status::Red
    }
}
