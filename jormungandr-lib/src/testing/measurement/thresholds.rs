use super::Status;
use std::{
    cmp::{Ordering, PartialEq, PartialOrd},
    fmt,
    time::Duration,
};

#[derive(Clone, Debug)]
pub struct Thresholds<T> {
    inner_thresholds: Vec<(Status, T)>,
    max: T,
}

impl<T: PartialOrd + Clone> Thresholds<T> {
    pub fn thresholds(&self) -> &Vec<(Status, T)> {
        &self.inner_thresholds
    }

    pub fn max(&self) -> T {
        self.max.clone()
    }

    pub fn green_threshold(&self) -> T {
        self.thresholds()
            .iter()
            .find(|(x, _)| *x == Status::Green)
            .expect("cannot find green threshold")
            .1
            .clone()
    }

    pub fn yellow_threshold(&self) -> T {
        self.thresholds()
            .iter()
            .find(|(x, _)| *x == Status::Yellow)
            .expect("cannot find green threshold")
            .1
            .clone()
    }

    pub fn red_threshold(&self) -> T {
        self.thresholds()
            .iter()
            .find(|(x, _)| *x == Status::Red)
            .expect("cannot find red threshold")
            .1
            .clone()
    }
}

impl Thresholds<Duration> {
    pub fn new(green: Duration, yellow: Duration, red: Duration, max: Duration) -> Self {
        Self {
            inner_thresholds: vec![
                (Status::Green, green),
                (Status::Yellow, yellow),
                (Status::Red, red),
            ],
            max: max,
        }
    }

    pub fn status(&self, actual: Duration) -> Status {
        let green = self.green_threshold();
        let yellow = self.yellow_threshold();

        if actual <= green {
            return Status::Green;
        }
        if actual <= yellow {
            return Status::Yellow;
        }
        Status::Red
    }
}

impl fmt::Display for Thresholds<Duration> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Green: {} s. Yellow: {} s. Red: {} s. Abort after: {} s",
            self.green_threshold().as_secs(),
            self.yellow_threshold().as_secs(),
            self.red_threshold().as_secs(),
            self.max().as_secs()
        )
    }
}

impl Thresholds<u64> {
    pub fn new(green: u64, yellow: u64, red: u64, max: u64) -> Self {
        Self {
            inner_thresholds: vec![
                (Status::Green, green),
                (Status::Yellow, yellow),
                (Status::Red, red),
            ],
            max: max,
        }
    }

    pub fn status(&self, actual: u64) -> Status {
        let green = self.green_threshold();
        let yellow = self.yellow_threshold();

        if actual >= green {
            return Status::Green;
        }
        if actual >= yellow {
            return Status::Yellow;
        }
        Status::Red
    }
}

impl fmt::Display for Thresholds<u64> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Max: {}. Green: {}. Yellow: {}. Red: {}.",
            self.max(),
            self.green_threshold(),
            self.yellow_threshold(),
            self.red_threshold(),
        )
    }
}

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
        write!(f, "{}", self.0.as_secs())
    }
}

impl Endurance {
    pub fn as_secs(&self) -> u64 {
        self.0.as_secs()
    }
}

impl Thresholds<Endurance> {
    pub fn new(green: Endurance, yellow: Endurance, red: Endurance, max: Endurance) -> Self {
        Self {
            inner_thresholds: vec![
                (Status::Green, green),
                (Status::Yellow, yellow),
                (Status::Red, red),
            ],
            max: max,
        }
    }

    pub fn status(&self, actual: Endurance) -> Status {
        let green = self.green_threshold();
        let yellow = self.yellow_threshold();

        if actual >= green {
            return Status::Green;
        }
        if actual >= yellow {
            return Status::Yellow;
        }
        Status::Red
    }
}

impl fmt::Display for Thresholds<Endurance> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Green: {} s. Yellow: {} s. Red: {} s. Max endurance: {} s",
            self.green_threshold().as_secs(),
            self.yellow_threshold().as_secs(),
            self.red_threshold().as_secs(),
            self.max().as_secs()
        )
    }
}
