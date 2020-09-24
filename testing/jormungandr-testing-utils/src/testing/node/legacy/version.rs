use std::{cmp::Ordering, fmt, num::ParseIntError, str::FromStr};
pub use semver::Version;

pub const fn version_0_8_19() -> Version {
    Version::new(0, 8, 19)
}