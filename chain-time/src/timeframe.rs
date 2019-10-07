use crate::timeline::Timeline;
use std::time::{Duration, SystemTime};

/// Identify a slot in a *specific* timeframe
///
/// The slots are not comparable to others slots made on a
/// different time frame
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Slot(pub(crate) u64);

impl Into<Slot> for u64 {
    fn into(self) -> Slot {
        Slot(self)
    }
}

impl From<Slot> for u64 {
    fn from(s: Slot) -> u64 {
        s.0
    }
}

/// Identify a slot in a specific timeframe and a leftover duration
#[derive(Debug)]
pub struct SlotAndDuration {
    pub slot: Slot,
    /// The offset of a specific time frame in
    pub offset: Duration,
}

/// Time frame which is a timeline that is configured to be split in discrete slots
#[derive(Debug, Clone)]
pub struct TimeFrame {
    timeline: Timeline,
    pub(crate) slot_offset: Slot,
    slot_duration: SlotDuration,
}

/// Duration of a slot
///
/// For now we only supports duration down to the seconds
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SlotDuration(u64);

impl SlotDuration {
    pub fn from_secs(seconds: u32) -> Self {
        assert!(seconds < 600);
        SlotDuration(seconds as u64)
    }

    pub fn to_duration(&self) -> Duration {
        Duration::from_secs(self.0)
    }
}

impl TimeFrame {
    /// Create a new time frame with a specific slot size
    ///
    /// ```text
    ///
    /// 0        1        2        3        4        5
    /// x--------x--------x--------x--------x--------x  frame ticking at per_slot
    ///
    /// ^
    /// |
    /// timeline
    /// ```
    ///
    pub fn new(timeline: Timeline, per_slot: SlotDuration) -> Self {
        TimeFrame {
            timeline,
            slot_offset: Slot(0),
            slot_duration: per_slot,
        }
    }

    /// Change time frame at a specific slot
    ///
    /// Note this also change the beginning of this time frame, to start
    ///
    /// ```text
    /// 0        1        2        3        4        5
    /// x--------x--------┳--------x--------x--------x  frame ticking at SlotDuration::from_secs(9)
    ///                   |
    ///                   ┕---x---x---x---x---x         returned frame
    ///                   2   3   4   5   6   7
    ///                   ↑
    ///                   |
    ///                   frame.change_frame(Slot(2), SlotDuration::from_secs(4))
    /// ```
    ///
    pub fn change_frame(&self, slot: Slot, duration_per_slot: SlotDuration) -> Self {
        let d = Duration::from_secs(slot.0 * self.slot_duration.0);
        let new_timeline = self.timeline.advance(d);
        TimeFrame {
            timeline: new_timeline,
            slot_offset: Slot(self.slot_offset.0 + slot.0),
            slot_duration: duration_per_slot,
        }
    }

    pub fn slot0(&self) -> Slot {
        Slot(self.slot_offset.0)
    }

    /// Given a system time get the slot and associated duration leftover
    pub fn slot_at_precise(&self, at: &SystemTime) -> Option<SlotAndDuration> {
        match self.timeline.differential(at) {
            None => None,
            Some(t) => {
                let slot_nb = t.0.as_secs() / self.slot_duration.0;
                let e = slot_nb * self.slot_duration.0;
                let d = t.0 - Duration::from_secs(e); // cannot wrap
                Some(SlotAndDuration {
                    slot: Slot(self.slot_offset.0 + slot_nb),
                    offset: d,
                })
            }
        }
    }

    /// Get the slot associated with the given system time.
    ///
    /// It returns None if the system time doesn't represent a valid slot in this time frame, for
    /// example if the system time is before the time frame starting point.
    pub fn slot_at(&self, at: &SystemTime) -> Option<Slot> {
        match self.timeline.differential(at) {
            None => None,
            Some(t) => {
                let slot_nb = t.0.as_secs() / self.slot_duration.0;
                Some(Slot(self.slot_offset.0 + slot_nb))
            }
        }
    }

    /// Get the system time associated with a slot on a specific timeframe
    ///
    /// Note if the slot is not supposed to be in this reference frame, then
    /// None is returned
    pub fn slot_to_systemtime(&self, slot: Slot) -> Option<SystemTime> {
        match slot.0.checked_sub(self.slot_offset.0) {
            None => None,
            Some(sd) => Some(self.timeline.0 + Duration::from_secs(sd * self.slot_duration.0)),
        }
    }

    /// Returns slot duration value.
    pub fn slot_duration(&self) -> u64 {
        self.slot_duration.0
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::timeline::Timeline;

    #[test]
    pub fn it_works() {
        let now = SystemTime::now();
        let t0 = Timeline::new(now);

        let f0 = SlotDuration::from_secs(5);

        let tf0 = TimeFrame::new(t0, f0);

        {
            let expected_slot = Slot(16);
            let x = now + Duration::from_secs(expected_slot.0 * f0.0);
            assert_eq!(tf0.slot_at(&x), Some(expected_slot));
        }

        let f1 = SlotDuration::from_secs(2);
        let tf1_start = now + Duration::from_secs(10);
        let s0 = tf0.slot_at(&tf1_start);
        assert_eq!(s0, Some(Slot(2)));
        let s0 = s0.unwrap();

        let tf1 = tf0.change_frame(s0, f1);

        assert_eq!(tf1.slot_at(&tf1_start), Some(Slot(2)));
        assert_eq!(tf1.slot_at(&now), None);

        let t2 = tf1_start + Duration::from_secs(10);
        assert_eq!(tf1.slot_at(&t2), Some(Slot(7)));

        assert_eq!(tf0.slot_at(&t2), Some(Slot(4)));
    }
}
