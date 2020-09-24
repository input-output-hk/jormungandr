pub use semver::Version;
use std::{cmp::Ordering, fmt, num::ParseIntError, str::FromStr};

pub fn version_0_8_19() -> Version {
    Version::new(0, 8, 19)
}
