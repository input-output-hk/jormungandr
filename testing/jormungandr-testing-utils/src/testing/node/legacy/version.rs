use std::{cmp::Ordering, fmt, num::ParseIntError, str::FromStr};

pub const fn version_0_8_19() -> Version {
    Version::new(0, 8, 19)
}

#[derive(Eq, Debug, Copy, Clone)]
pub struct Version {
    major: u32,
    minor: u32,
    patch: u32,
}

impl Version {
    pub const fn new(major: u32, minor: u32, patch: u32) -> Self {
        Version {
            major,
            minor,
            patch,
        }
    }
}

impl FromStr for Version {
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        //remove first char
        let version_str = s.chars().next().map(|c| &s[c.len_utf8()..]).unwrap();
        let mut tokens = version_str.split('.');
        let major: u32 = tokens.next().unwrap().parse().unwrap();
        let minor: u32 = tokens.next().unwrap().parse().unwrap();
        let patch: u32 = tokens.next().unwrap().parse().unwrap();
        Ok(Version {
            major,
            minor,
            patch,
        })
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        self.major
            .cmp(&other.major)
            .then(self.minor.cmp(&other.minor))
            .then(self.patch.cmp(&other.patch))
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "(v{}.{}.{})", self.major, self.minor, self.patch)
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Version {
    fn eq(&self, other: &Self) -> bool {
        self.major == other.major && self.minor == other.minor && self.patch == other.patch
    }
}
