#![allow(dead_code)]

use std::time::SystemTime;

pub fn get_current_time_epoch() {
    SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
