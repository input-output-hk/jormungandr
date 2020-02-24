use crate::testing::measurement::{marker::Timestamp, status::Status, thresholds::Thresholds};
use std::{cmp::Ordering, fmt, time::Duration};

#[derive(Clone, Debug)]
pub struct Endurance(Duration);

impl From<Duration> for Endurance {
    fn from(duration: Duration) -> Self {
        Endurance(duration)
    }
}

impl Into<Duration> for Endurance {
    fn into(self) -> Duration {
        self.0
    }
}

impl PartialOrd for Endurance {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.0.cmp(&other.0))
    }
}

impl PartialEq for Endurance {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl fmt::Display for Endurance {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} s.", self.0.as_millis() as f32 / 1000.0)
    }
}

impl Endurance {
    pub fn new(start_time: &Timestamp, end_time: &Timestamp) -> Self {
        Self(end_time.duration_since(&start_time))
    }

    pub fn as_secs(&self) -> u64 {
        self.0.as_secs()
    }

    pub fn against(&self, thesholds: &Thresholds<Self>) -> Status {
        let green = thesholds.green_threshold();
        let yellow = thesholds.yellow_threshold();

        if *self >= green {
            return Status::Green;
        }
        if *self >= yellow {
            return Status::Yellow;
        }
        Status::Red
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duration_since() {
        let start_time: Timestamp = "2020-02-20T17:15:13.596834700+01:00".parse().unwrap();
        let end_time: Timestamp = "2020-02-20T17:15:14.606834700+01:00".parse().unwrap();

        Endurance::new(&start_time, &end_time);
    }
}
