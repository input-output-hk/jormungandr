mod status;
mod thresholds;

pub use status::Status;
use std::{cmp, fmt, time::Duration};
pub use thresholds::{Endurance, Thresholds};

#[derive(Clone, Debug)]
pub struct Measurement<T> {
    info: String,
    actual: T,
    thresholds: Thresholds<T>,
}

impl<T: cmp::PartialOrd + Clone> Measurement<T> {
    pub fn new(info: String, actual: T, thresholds: Thresholds<T>) -> Self {
        Self {
            info,
            actual,
            thresholds,
        }
    }

    pub fn info(&self) -> String {
        self.info.clone()
    }

    pub fn actual(&self) -> T {
        self.actual.clone()
    }

    pub fn thresholds(&self) -> Thresholds<T> {
        self.thresholds.clone()
    }
}

impl Measurement<Duration> {
    pub fn result(&self) -> Status {
        self.thresholds().status(self.actual.clone())
    }
}

impl fmt::Display for Measurement<Duration> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Measurement: {}. Result: {}. Actual: {:.3} s. Thresholds: {}",
            self.info(),
            self.result().to_string(),
            self.actual.as_millis() as f32 / 1000.0,
            self.thresholds
        )
    }
}

impl Measurement<u64> {
    pub fn result(&self) -> Status {
        self.thresholds().status(self.actual.clone())
    }
}

impl fmt::Display for Measurement<u64> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Measurement: {}. Result: {}. Actual: {}. Thresholds: {}",
            self.info(),
            self.result().to_string(),
            self.actual,
            self.thresholds
        )
    }
}

impl Measurement<Endurance> {
    pub fn result(&self) -> Status {
        self.thresholds().status(self.actual.clone())
    }
}

impl fmt::Display for Measurement<Endurance> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let actual: Duration = self.actual.clone().into();
        write!(
            f,
            "Measurement: {}. Result: {}. Actual: {}. Thresholds: {}",
            self.info(),
            self.result().to_string(),
            actual.as_millis() as f32 / 1000.0,
            self.thresholds
        )
    }
}

pub fn thresholds_for_transaction_counter(counter: u64) -> Thresholds<u64> {
    let green = (counter / 2) as u64;
    let yellow = (counter / 3) as u64;
    let red = (counter / 4) as u64;
    Thresholds::<u64>::new(green, yellow, red, counter)
}

pub fn thresholds_for_transaction_duration(duration: Duration) -> Thresholds<Duration> {
    let green = Duration::from_secs(duration.as_secs() / 2);
    let yellow = Duration::from_secs(duration.as_secs() / 3);
    let red = Duration::from_secs(duration.as_secs() / 4);
    Thresholds::<Duration>::new(green, yellow, red, duration)
}

pub fn thresholds_for_transaction_endurance(secs: u64) -> Thresholds<Endurance> {
    let green = Duration::from_secs(secs / 2);
    let yellow = Duration::from_secs(secs / 3);
    let red = Duration::from_secs(secs / 4);
    Thresholds::<Endurance>::new(
        green.into(),
        yellow.into(),
        red.into(),
        Duration::from_secs(secs).into(),
    )
}
