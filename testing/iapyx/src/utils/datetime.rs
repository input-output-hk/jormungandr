use chrono::{DateTime, NaiveDateTime, Utc};

pub fn unix_timestamp_to_datetime(timestamp: i64) -> DateTime<Utc> {
    DateTime::from_utc(NaiveDateTime::from_timestamp(timestamp, 0), Utc)
}
