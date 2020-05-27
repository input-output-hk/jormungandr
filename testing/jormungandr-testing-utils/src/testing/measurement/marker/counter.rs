use std::ops::{Add, Div, Sub};

#[derive(Default, Copy, Clone)]
pub struct Counter(u32);

impl Counter {
    pub fn new() -> Self {
        Counter::from(0u32)
    }
}

impl From<u32> for Counter {
    fn from(from: u32) -> Self {
        Counter(from)
    }
}

impl Into<u32> for Counter {
    fn into(self) -> u32 {
        self.0
    }
}

impl Sub for Counter {
    type Output = Counter;

    fn sub(self, other: Counter) -> Counter {
        Counter(self.0 - other.0)
    }
}

impl Add for Counter {
    type Output = Counter;

    fn add(self, other: Counter) -> Counter {
        Counter(self.0 + other.0)
    }
}

impl Div for Counter {
    type Output = Counter;

    fn div(self, other: Counter) -> Counter {
        Counter(self.0 / other.0)
    }
}
