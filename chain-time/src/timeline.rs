use crate::units::DurationSeconds;
use std::time::{Duration, SystemTime};

/// Represent a timeline with a specific start point rooted on earth time.
#[derive(Debug, Clone)]
pub struct Timeline(pub(crate) SystemTime);

/// Represent an offset in time units in the timeline
#[derive(Debug, Clone)]
pub struct TimeOffset(pub(crate) Duration);

/// Represent an offset in seconds in the timeline
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TimeOffsetSeconds(pub(crate) DurationSeconds);

impl From<SystemTime> for Timeline {
    fn from(s: SystemTime) -> Self {
        Timeline(s)
    }
}

impl From<DurationSeconds> for TimeOffsetSeconds {
    fn from(v: DurationSeconds) -> Self {
        TimeOffsetSeconds(v)
    }
}

impl From<TimeOffsetSeconds> for TimeOffset {
    fn from(v: TimeOffsetSeconds) -> TimeOffset {
        TimeOffset(v.0.into())
    }
}

impl From<TimeOffsetSeconds> for u64 {
    fn from(v: TimeOffsetSeconds) -> Self {
        v.0.into()
    }
}

impl Timeline {
    /// Create a new timeline, which is a time starting point
    pub fn new(start_time: SystemTime) -> Self {
        Timeline(start_time)
    }

    /// Return the duration since the creation of the timeline
    ///
    /// If the time is earlier than the start of this timeline,
    /// then None is returned.
    pub fn differential(&self, t: &SystemTime) -> Option<TimeOffset> {
        match t.duration_since(self.0) {
            Ok(duration) => Some(TimeOffset(duration)),
            Err(_) => None,
        }
    }

    /// Advance a timeline, and create a new timeline starting at
    /// timeline + duration
    pub fn advance(&self, d: Duration) -> Self {
        Timeline(self.0 + d)
    }
}
