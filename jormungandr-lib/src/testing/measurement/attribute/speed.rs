use crate::testing::measurement::{marker::Timestamp, status::Status, thresholds::Thresholds};
use std::{cmp::Ordering, fmt, time::Duration};

#[derive(Clone, Debug)]
pub struct Speed(Duration);

impl From<Duration> for Speed {
    fn from(duration: Duration) -> Self {
        Speed(duration)
    }
}

impl Into<Duration> for Speed {
    fn into(self) -> Duration {
        self.0
    }
}

impl PartialOrd for Speed {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.0.cmp(&other.0))
    }
}

impl PartialEq for Speed {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl fmt::Display for Speed {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} s.", self.0.as_millis() as f32 / 1000.0)
    }
}

impl Speed {
    pub fn new(start: &Timestamp, end: &Timestamp) -> Self {
        Self(end.duration_since(start))
    }

    pub fn as_secs(&self) -> u64 {
        self.0.as_secs()
    }

    pub fn against(&self, thresholds: &Thresholds<Self>) -> Status {
        let green = thresholds.green_threshold();
        let yellow = thresholds.yellow_threshold();

        if *self <= green {
            return Status::Green;
        }
        if *self <= yellow {
            return Status::Yellow;
        }
        Status::Red
    }
}
