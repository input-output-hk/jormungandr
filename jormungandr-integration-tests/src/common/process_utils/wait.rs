use std::time::Duration;

#[derive(Clone, Debug)]
pub struct Wait {
    sleep: Duration,
    attempts: u64,
}

impl Wait {
    pub fn new(sleep: Duration, attempts: u64) -> Self {
        Wait { sleep, attempts }
    }

    pub fn sleep(&self) -> Duration {
        self.sleep
    }

    pub fn attempts(&self) -> u64 {
        self.attempts
    }
}

impl Default for Wait {
    fn default() -> Self {
        Wait::new(Duration::from_secs(1), 5)
    }
}

impl From<Duration> for Wait {
    fn from(duration: Duration) -> Wait {
        let attempts = duration.as_secs();
        let sleep = 1;
        Wait::new(Duration::from_secs(sleep), attempts)
    }
}

pub struct WaitBuilder {
    sleep: Duration,
    attempts: u64,
}

impl WaitBuilder {
    pub fn new() -> Self {
        WaitBuilder {
            sleep: Duration::from_secs(1),
            attempts: 5,
        }
    }

    pub fn tries(&mut self, attempts: u64) -> &mut Self {
        self.attempts = attempts;
        self
    }

    pub fn sleep_between_tries(&mut self, sleep: u64) -> &mut Self {
        self.sleep = Duration::from_secs(sleep);
        self
    }

    pub fn build(&self) -> Wait {
        Wait::new(self.sleep, self.attempts)
    }
}
