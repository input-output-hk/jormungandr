use crate::testing::measurement::{marker::ResourcesUsage, status::Status, thresholds::Thresholds};
use std::{cmp::Ordering, fmt};

#[derive(Clone, Debug)]
pub struct Consumption {
    average_usage: ResourcesUsage,
}

impl fmt::Display for Consumption {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.average_usage)
    }
}

impl PartialEq for Consumption {
    fn eq(&self, other: &Self) -> bool {
        self.average_usage == other.average_usage
    }
}

impl PartialOrd for Consumption {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.average_usage
            .usage_indicator()
            .partial_cmp(&other.average_usage.usage_indicator())
    }
}

impl Consumption {
    pub fn new(markers: Vec<ResourcesUsage>) -> Self {
        let median_marker = Self::median(markers);
        Self {
            average_usage: median_marker,
        }
    }

    fn median(mut markers: Vec<ResourcesUsage>) -> ResourcesUsage {
        markers.sort();
        let mid = markers.len() / 2;
        markers.get(mid).unwrap().clone()
    }

    pub fn against(&self, thresholds: &Thresholds<Self>) -> Status {
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
