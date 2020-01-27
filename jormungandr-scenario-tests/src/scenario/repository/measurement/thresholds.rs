use crate::scenario::repository::MeasurementStatus;
use std::{fmt, time::Duration};

#[derive(Clone, Debug)]
pub struct MeasurementThresholds {
    pub thresholds: Vec<(MeasurementStatus, Duration)>,
    pub timeout: Duration,
}

impl MeasurementThresholds {
    pub fn new(green: u64, yellow: u64, red: u64, timeout: u64) -> Self {
        Self {
            thresholds: vec![
                (MeasurementStatus::Green, Duration::from_secs(green)),
                (MeasurementStatus::Yellow, Duration::from_secs(yellow)),
                (MeasurementStatus::Red, Duration::from_secs(red)),
            ],
            timeout: Duration::from_secs(timeout),
        }
    }

    pub fn timeout(&self) -> Duration {
        self.timeout.clone()
    }

    pub fn green_threshold(&self) -> Duration {
        self.thresholds
            .iter()
            .find(|(x, _)| *x == MeasurementStatus::Green)
            .expect("cannot find green threshold")
            .1
    }

    pub fn yellow_threshold(&self) -> Duration {
        self.thresholds
            .iter()
            .find(|(x, _)| *x == MeasurementStatus::Yellow)
            .expect("cannot find green threshold")
            .1
    }

    pub fn red_threshold(&self) -> Duration {
        self.thresholds
            .iter()
            .find(|(x, _)| *x == MeasurementStatus::Red)
            .expect("cannot find red threshold")
            .1
    }
    pub fn status(&self, actual: &Duration) -> MeasurementStatus {
        let green = self.green_threshold();
        let yellow = self.yellow_threshold();

        if *actual <= green {
            return MeasurementStatus::Green;
        }
        if *actual <= yellow {
            return MeasurementStatus::Yellow;
        }
        MeasurementStatus::Red
    }
}

impl fmt::Display for MeasurementThresholds {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Green: {} s. Yellow: {} s. Red: {} s. Abort after: {} s",
            self.green_threshold().as_secs(),
            self.yellow_threshold().as_secs(),
            self.red_threshold().as_secs(),
            self.timeout().as_secs()
        )
    }
}
