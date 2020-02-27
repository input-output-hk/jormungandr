use std::{
    fmt,
    str::FromStr,
    time::{Duration, SystemTime},
};

#[derive(Clone, Copy)]
pub struct Timestamp(SystemTime);

impl Timestamp {
    pub fn new() -> Self {
        Timestamp::from(SystemTime::now())
    }

    pub fn duration_since(&self, earlier: &Timestamp) -> Duration {
        let system_time: SystemTime = earlier.clone().into();
        self.0.duration_since(system_time).unwrap()
    }

    pub fn elapsed(&self) -> Duration {
        self.0.elapsed().unwrap()
    }
}

impl From<SystemTime> for Timestamp {
    fn from(from: SystemTime) -> Self {
        Timestamp(from)
    }
}

impl Into<SystemTime> for Timestamp {
    fn into(self) -> SystemTime {
        self.0
    }
}
impl FromStr for Timestamp {
    type Err = chrono::ParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let system_time: crate::time::SystemTime = s.parse().unwrap();
        let std_system_time: std::time::SystemTime = system_time.into();
        Ok(std_system_time.into())
    }
}

impl fmt::Debug for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.0)
    }
}
