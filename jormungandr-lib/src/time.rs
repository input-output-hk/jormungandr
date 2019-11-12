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
/// This type is meant to be easily converted between [`SystemTime`]
///
/// # Example
///
/// ```
/// # use jormungandr_lib::time::{SystemTime, SecondsSinceUnixEpoch};
///
/// let time = SystemTime::from(SecondsSinceUnixEpoch::MAX);
///
/// println!("max allowed time: {}", time);
/// // max allowed time: 4147-08-20T07:32:15+00:00
/// ```
///
/// [`SystemTime`]: ./struct.SystemTime.html
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct SecondsSinceUnixEpoch(pub(crate) u64);

/// time in seconds and nanoseconds since [UNIX Epoch]
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

impl SecondsSinceUnixEpoch {
    /// maximum authorized Time in seconds since unix epoch
    ///
    /// This value will take you up to the year 4147.
    pub const MAX: Self = SecondsSinceUnixEpoch(0x000_000F_FFFF_FFFF);

    pub fn now() -> Self {
        SystemTime::now().into()
    }
}

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

    pub fn duration_since(&self, earlier: SystemTime) -> Result<Duration, time::SystemTimeError> {
        self.0.duration_since(earlier.0).map(Duration)
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

/* --------------------- Default ------------------------------------------- */

impl Default for SecondsSinceUnixEpoch {
    fn default() -> SecondsSinceUnixEpoch {
        SecondsSinceUnixEpoch::now()
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

impl fmt::Display for SecondsSinceUnixEpoch {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl str::FromStr for SecondsSinceUnixEpoch {
    type Err = std::num::ParseIntError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse().map(SecondsSinceUnixEpoch)
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

impl From<time::SystemTime> for SystemTime {
    fn from(system_time: time::SystemTime) -> Self {
        SystemTime(system_time)
    }
}

impl From<SystemTime> for time::SystemTime {
    fn from(system_time: SystemTime) -> Self {
        system_time.0
    }
}

impl From<time::SystemTime> for SecondsSinceUnixEpoch {
    fn from(system_time: time::SystemTime) -> Self {
        system_time
            .duration_since(time::UNIX_EPOCH)
            // duration since UNIX EPOCH will never go beyond boundaries
            .map(|duration| duration.as_secs())
            .map(SecondsSinceUnixEpoch::from_secs)
            .unwrap()
    }
}

impl SystemTime {
    pub fn from_secs_since_epoch(secs: u64) -> Self {
        // here we can safely unwrap as we are adding from UNIX_EPOCH (0)
        // and SecondsSinceUnixEpoch is always a positive integer
        // and seconds will always be within bounds
        time::UNIX_EPOCH
            .checked_add(time::Duration::from_secs(secs))
            .unwrap()
            .into()
    }
}

impl SecondsSinceUnixEpoch {
    pub fn from_secs(secs: u64) -> Self {
        SecondsSinceUnixEpoch(secs)
    }

    pub fn to_secs(self) -> u64 {
        self.0
    }
}

impl From<time::Duration> for Duration {
    fn from(duration: time::Duration) -> Self {
        Duration(duration)
    }
}

impl From<Duration> for time::Duration {
    fn from(Duration(duration): Duration) -> Self {
        duration
    }
}

impl From<SecondsSinceUnixEpoch> for SystemTime {
    fn from(seconds: SecondsSinceUnixEpoch) -> SystemTime {
        SystemTime::from_secs_since_epoch(seconds.0)
    }
}

impl From<SystemTime> for SecondsSinceUnixEpoch {
    fn from(system_time: SystemTime) -> SecondsSinceUnixEpoch {
        system_time.0.into()
    }
}

/* ------------------- Serde ----------------------------------------------- */

impl<'de> Deserialize<'de> for SecondsSinceUnixEpoch {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::{self, Visitor};
        struct SecondsSinceUnixEpochVisitor;
        impl<'de> Visitor<'de> for SecondsSinceUnixEpochVisitor {
            type Value = SecondsSinceUnixEpoch;
            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                write!(
                    formatter,
                    "Seconds since unix epoch up to '{}' ({})",
                    SecondsSinceUnixEpoch::MAX,
                    SystemTime::from(SecondsSinceUnixEpoch::MAX),
                )
            }

            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                let seconds = SecondsSinceUnixEpoch(v);

                if seconds > SecondsSinceUnixEpoch::MAX {
                    Err(E::custom("Time value is way too far in the future"))
                } else {
                    Ok(seconds)
                }
            }
        }
        deserializer.deserialize_u64(SecondsSinceUnixEpochVisitor)
    }
}

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
        use serde::de::{self, Visitor};
        struct SystemTimeVisitor;
        impl<'de> Visitor<'de> for SystemTimeVisitor {
            type Value = SystemTime;
            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("SystemTime in ISO8601 format")
            }

            fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                s.parse().map_err(E::custom)
            }
        }

        if deserializer.is_human_readable() {
            deserializer.deserialize_str(SystemTimeVisitor)
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
        use serde::de::{self, Visitor};
        struct DurationVisitor;
        impl<'de> Visitor<'de> for DurationVisitor {
            type Value = Duration;
            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("duration in the form of '10days 7h 2m 45s'")
            }

            fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                s.parse().map_err(E::custom)
            }
        }

        if deserializer.is_human_readable() {
            deserializer.deserialize_str(DurationVisitor)
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
            unimplemented!("non human readable format not supported for LocalDateTime")
        }
    }
}

impl<'de> Deserialize<'de> for LocalDateTime {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::{self, Visitor};
        struct LocalDateTimeVisitor;
        impl<'de> Visitor<'de> for LocalDateTimeVisitor {
            type Value = LocalDateTime;
            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("local date time, in RFC2822 format")
            }

            fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                s.parse().map_err(E::custom)
            }
        }

        assert!(
            deserializer.is_human_readable(),
            "LocalDateTime only supported for human readable format"
        );
        deserializer.deserialize_str(LocalDateTimeVisitor)
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

    impl Arbitrary for SecondsSinceUnixEpoch {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            SecondsSinceUnixEpoch(u64::arbitrary(g) % SecondsSinceUnixEpoch::MAX.0)
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

        fn seconds_since_unix_epoch_serde_human_readable_encode_decode(seconds: SecondsSinceUnixEpoch) -> bool {
            let s = serde_yaml::to_string(&seconds).unwrap();
            let seconds_dec: SecondsSinceUnixEpoch = serde_yaml::from_str(&s).unwrap();

            seconds == seconds_dec
        }

        fn seconds_since_unix_epoch_serde_binary_readable_encode_decode(seconds: SecondsSinceUnixEpoch) -> bool {
            let s = bincode::serialize(&seconds).unwrap();
            let seconds_dec: SecondsSinceUnixEpoch = bincode::deserialize(&s).unwrap();

            seconds == seconds_dec
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

    #[test]
    fn check_conversions_seconds_since_epoch_between_system_time_boundaries() {
        let seconds_since_epoch = SecondsSinceUnixEpoch::MAX;

        let system_time = SystemTime::from(seconds_since_epoch);

        let seconds_since_epoch_2 = SecondsSinceUnixEpoch::from(system_time);

        assert_eq!(seconds_since_epoch, seconds_since_epoch_2);
    }

    #[test]
    fn seconds_since_unix_epoch_serde_human_readable() {
        let duration = SecondsSinceUnixEpoch(9982716);

        assert_eq!(serde_yaml::to_string(&duration).unwrap(), "---\n9982716")
    }

    #[test]
    #[should_panic]
    fn out_of_bound_seconds_since_unix_epoch_serde_human_readable_fail() {
        let invalid_yaml = format!("---\n{}", SecondsSinceUnixEpoch::MAX.0 + 1);

        let _: SecondsSinceUnixEpoch = serde_yaml::from_str(&invalid_yaml).unwrap();
    }
}
