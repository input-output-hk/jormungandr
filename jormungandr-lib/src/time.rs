//! Time and related data structures
//!
//! this module exports three different components of time:
//! [`SystemTime`], [`LocalDateTime`] and [`Duration`].
//!
//! [`SystemTime`]: ./struct.SystemTime.html
//! [`LocalDateTime`]: ./struct.LocalDateTime.html
//! [`Duration`]: ./struct.Duration.html

use chrono::prelude::{DateTime, Local, TimeZone as _, Utc};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{convert::TryFrom, fmt, str, time};

/// time in seconds since [UNIX Epoch]
///
/// The human readable formatting is [ISO8601] compliant.
///
/// # Example
///
/// ```
/// # use jormungandr_lib::time::SystemTime;
///
/// let time = SystemTime::now();
///
/// println!("now: {}", time);
/// // now: 2019-06-17T18:17:20.417032+00:00
/// ```
///
/// [ISO8601]: https://en.wikipedia.org/wiki/ISO_8601
/// [`LocalDateTime`]: ./struct.LocalDateTime.html
/// [UNIX Epoch]: https://en.wikipedia.org/wiki/Unix_time
///
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SystemTime(time::SystemTime);

/// local date and time. While the [`SystemTime`] will give us a number of seconds
/// since [UNIX Epoch] this will take into account the locality of the caller, taking
/// into account daylight saving.
///
/// # Example
///
/// ```
/// # use jormungandr_lib::time::LocalDateTime;
///
/// let time = LocalDateTime::now();
///
/// println!("now: {}", time);
/// // now: Mon, 17 Jun 2019 20:19:29 +0200
/// ```
///
/// [`SystemTime`]: ./struct.SystemTime.html
/// [UNIX Epoch]: https://en.wikipedia.org/wiki/Unix_time
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LocalDateTime(DateTime<Local>);

/// Length of time between 2 events.
///
/// # Example
///
/// ```
/// # use jormungandr_lib::time::Duration;
///
/// let duration = Duration::new(9289, 200000000);
///
/// println!("started: {}", duration);
/// // started: 2h 34m 49s 200ms
/// ```
///
///
/// [UNIX Epoch]: https://en.wikipedia.org/wiki/Unix_time
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Duration(time::Duration);

impl SystemTime {
    /// get the current time in seconds since [UNIX Epoch]
    ///
    /// [UNIX Epoch]: https://en.wikipedia.org/wiki/Unix_time
    #[inline]
    pub fn now() -> Self {
        SystemTime(time::SystemTime::now())
    }

    fn utc_date_time(&self) -> DateTime<Utc> {
        let timestamps = self.0.duration_since(time::UNIX_EPOCH).unwrap();
        Utc.timestamp(timestamps.as_secs() as i64, timestamps.subsec_nanos())
    }
}

impl LocalDateTime {
    #[inline]
    pub fn now() -> Self {
        LocalDateTime(Local::now())
    }
}

impl Duration {
    #[inline]
    pub fn new(secs: u64, nanos: u32) -> Self {
        Duration(time::Duration::new(secs, nanos))
    }
}

/* --------------------- Display ------------------------------------------- */

impl fmt::Display for Duration {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        humantime::Duration::from(self.0.clone()).fmt(f)
    }
}

impl str::FromStr for Duration {
    type Err = humantime::DurationError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let duration = humantime::parse_duration(s)?;
        Ok(Duration(duration))
    }
}

impl fmt::Display for SystemTime {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.utc_date_time().to_rfc3339().fmt(f)
    }
}

impl str::FromStr for SystemTime {
    type Err = chrono::ParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let dt: DateTime<chrono::FixedOffset> = DateTime::parse_from_rfc3339(s)?;
        let seconds = dt.timestamp() as u64;
        let nsecs = dt.timestamp_subsec_nanos();

        let elapsed = time::Duration::new(seconds, nsecs);

        let time = time::SystemTime::UNIX_EPOCH.checked_add(elapsed).unwrap();

        Ok(SystemTime(time))
    }
}

impl fmt::Display for LocalDateTime {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.to_rfc2822().fmt(f)
    }
}

impl str::FromStr for LocalDateTime {
    type Err = chrono::ParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let dt: DateTime<chrono::FixedOffset> = DateTime::parse_from_rfc2822(s)?;
        let seconds = dt.timestamp();
        let nsecs = dt.timestamp_subsec_nanos();

        let time = Local.timestamp(seconds, nsecs);

        Ok(LocalDateTime(time))
    }
}

/* --------------------- AsRef --------------------------------------------- */

impl AsRef<time::Duration> for Duration {
    fn as_ref(&self) -> &time::Duration {
        &self.0
    }
}

impl AsRef<time::SystemTime> for SystemTime {
    fn as_ref(&self) -> &time::SystemTime {
        &self.0
    }
}

impl AsRef<chrono::DateTime<chrono::Local>> for LocalDateTime {
    fn as_ref(&self) -> &chrono::DateTime<chrono::Local> {
        &self.0
    }
}

/* --------------------- Conversion ---------------------------------------- */

impl TryFrom<SystemTime> for LocalDateTime {
    type Error = time::SystemTimeError;
    fn try_from(system_time: SystemTime) -> Result<Self, Self::Error> {
        let timestamps = system_time.0.duration_since(time::UNIX_EPOCH)?;
        let local = Local.timestamp(timestamps.as_secs() as i64, timestamps.subsec_nanos());
        Ok(LocalDateTime(local))
    }
}

impl From<time::Duration> for Duration {
    fn from(duration: time::Duration) -> Self {
        Duration(duration)
    }
}

/* ------------------- Serde ----------------------------------------------- */

impl Serialize for SystemTime {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if serializer.is_human_readable() {
            self.to_string().serialize(serializer)
        } else {
            self.0.serialize(serializer)
        }
    }
}

impl<'de> Deserialize<'de> for SystemTime {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        if deserializer.is_human_readable() {
            use std::str::FromStr as _;

            let s = String::deserialize(deserializer)?;
            SystemTime::from_str(&s).map_err(<D::Error as serde::de::Error>::custom)
        } else {
            time::SystemTime::deserialize(deserializer).map(SystemTime)
        }
    }
}

impl Serialize for Duration {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if serializer.is_human_readable() {
            self.to_string().serialize(serializer)
        } else {
            self.0.serialize(serializer)
        }
    }
}

impl<'de> Deserialize<'de> for Duration {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        if deserializer.is_human_readable() {
            use std::str::FromStr as _;

            let s = String::deserialize(deserializer)?;
            Duration::from_str(&s).map_err(<D::Error as serde::de::Error>::custom)
        } else {
            time::Duration::deserialize(deserializer).map(Duration)
        }
    }
}

impl Serialize for LocalDateTime {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if serializer.is_human_readable() {
            self.to_string().serialize(serializer)
        } else {
            unimplemented!()
        }
    }
}

impl<'de> Deserialize<'de> for LocalDateTime {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        if deserializer.is_human_readable() {
            use std::str::FromStr as _;

            let s = String::deserialize(deserializer)?;
            LocalDateTime::from_str(&s).map_err(<D::Error as serde::de::Error>::custom)
        } else {
            unimplemented!()
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use quickcheck::{Arbitrary, Gen};
    use std::time;

    impl Arbitrary for Duration {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            Duration::new(u64::arbitrary(g), u32::arbitrary(g))
        }
    }

    impl Arbitrary for SystemTime {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            let secs = u64::arbitrary(g) % 0xFF_FFFF_FFFF;
            let nanos = u32::arbitrary(g) % 999_999_999;
            SystemTime(
                time::SystemTime::UNIX_EPOCH
                    .checked_add(time::Duration::new(secs, nanos))
                    .unwrap(),
            )
        }
    }

    impl Arbitrary for LocalDateTime {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            // no nanoseconds for the local date time as this is not displayed
            const NSECS: u32 = 0;

            let secs = i64::arbitrary(g) % 0xFF_FFFF_FFFF;
            LocalDateTime(Local.timestamp(secs, NSECS))
        }
    }

    quickcheck! {
        fn system_time_display_parse(time: SystemTime) -> bool {
            let s = time.to_string();
            let time_dec: SystemTime = s.parse().unwrap();

            time == time_dec
        }

        fn system_time_serde_human_readable_encode_decode(time: SystemTime) -> bool {
            let s = serde_yaml::to_string(&time).unwrap();
            let time_dec: SystemTime = serde_yaml::from_str(&s).unwrap();

            time == time_dec
        }

        fn system_time_serde_binary_readable_encode_decode(time: SystemTime) -> bool {
            let s = bincode::serialize(&time).unwrap();
            let time_dec: SystemTime = bincode::deserialize(&s).unwrap();

            time == time_dec
        }

        fn local_date_time_display_parse(time: LocalDateTime) -> bool {
            let s = time.to_string();
            let time_dec: LocalDateTime = s.parse().unwrap();

            dbg!(s);
            dbg!(&time_dec);

            time == time_dec
        }

        fn local_date_time_serde_human_readable_encode_decode(time: LocalDateTime) -> bool {
            let s = serde_yaml::to_string(&time).unwrap();
            let time_dec: LocalDateTime = serde_yaml::from_str(&s).unwrap();

            time == time_dec
        }

        fn duration_display_parse(duration: Duration) -> bool {
            let s = duration.to_string();
            let duration_dec: Duration = s.parse().unwrap();

            duration == duration_dec
        }

        fn duration_serde_human_readable_encode_decode(duration: Duration) -> bool {
            let s = serde_yaml::to_string(&duration).unwrap();
            let duration_dec: Duration = serde_yaml::from_str(&s).unwrap();

            duration == duration_dec
        }

        fn duration_serde_binary_readable_encode_decode(duration: Duration) -> bool {
            let s = bincode::serialize(&duration).unwrap();
            let duration_dec: Duration = bincode::deserialize(&s).unwrap();

            duration == duration_dec
        }

    }

    #[test]
    fn system_time_display_iso8601() {
        let epoch = SystemTime(time::UNIX_EPOCH);

        assert_eq!(epoch.to_string(), "1970-01-01T00:00:00+00:00")
    }

    #[test]
    fn system_time_serde_human_readable() {
        let epoch = SystemTime(time::UNIX_EPOCH);

        assert_eq!(
            serde_yaml::to_string(&epoch).unwrap(),
            "---\n\"1970-01-01T00:00:00+00:00\""
        )
    }

    #[test]
    fn local_date_time_display_rfc_2822() {
        let local = LocalDateTime(Local.ymd(2017, 08, 17).and_hms(11, 59, 42));

        assert!(local.to_string().starts_with("Thu, 17 Aug 2017 "));
    }

    #[test]
    fn local_date_time_serde_human_readable() {
        let local = LocalDateTime(Local.ymd(2017, 08, 17).and_hms(11, 59, 42));

        assert!(serde_yaml::to_string(&local)
            .unwrap()
            .starts_with("---\n\"Thu, 17 Aug 2017 "))
    }

    #[test]
    fn duration_display_readable() {
        let duration = Duration(time::Duration::new(928237, 1129000));

        assert_eq!(duration.to_string(), "10days 17h 50m 37s 1ms 129us")
    }

    #[test]
    fn duration_serde_human_readable() {
        let duration = Duration(time::Duration::new(928237, 1129000));

        assert_eq!(
            serde_yaml::to_string(&duration).unwrap(),
            "---\n10days 17h 50m 37s 1ms 129us"
        )
    }

}
