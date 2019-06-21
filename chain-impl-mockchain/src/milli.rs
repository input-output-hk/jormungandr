use std::str::FromStr;
use std::{fmt, iter};

const MILLI_MULTIPLIER: u64 = 1000;

#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub struct Milli(u64);

impl Milli {
    pub const ZERO: Milli = Milli(0);
    pub const HALF: Milli = Milli(MILLI_MULTIPLIER / 2);
    pub const ONE: Milli = Milli(MILLI_MULTIPLIER);

    pub const fn from_millis(value: u64) -> Self {
        Milli(value)
    }

    pub fn to_millis(self) -> u64 {
        self.0
    }

    pub fn to_float(self) -> f64 {
        self.0 as f64 / MILLI_MULTIPLIER as f64
    }
}

impl FromStr for Milli {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Err("Input string empty".to_string());
        }
        let mut parts = s.splitn(2, '.');
        let integrals_iter = parts.next().unwrap_or("").chars();
        let millis_iter = parts
            .next()
            .unwrap_or("")
            .chars()
            .chain(iter::repeat('0'))
            .take(3);
        integrals_iter
            .chain(millis_iter)
            .collect::<String>()
            .parse()
            .map(Milli)
            .map_err(|_| "Failed to parse milli".to_string())
    }
}

impl fmt::Display for Milli {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(
            f,
            "{}.{:0>3}",
            self.0 / MILLI_MULTIPLIER,
            self.0 % MILLI_MULTIPLIER
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quickcheck::{Arbitrary, Gen, TestResult};

    fn assert_from_str(input: &str, expected_milli: u64) {
        let expected = Milli::from_millis(expected_milli);

        let result = Milli::from_str(input);

        let actual = result.expect(&format!("Failed to parse for input {}", input));
        assert_eq!(actual, expected, "Invalid result for input {}", input);
    }

    fn refute_from_str(input: &str) {
        let result = Milli::from_str(input);

        assert!(result.is_err(), "Parsing did not fail for input {}", input);
    }

    #[test]
    fn from_str() {
        refute_from_str("");
        refute_from_str("X");
        assert_from_str("0", 0);
        assert_from_str("1", 1000);
        assert_from_str(".", 0);
        assert_from_str("1.", 1000);
        assert_from_str(".1", 100);
        assert_from_str(".100", 100);
        assert_from_str(".001", 1);
        assert_from_str(".0009", 0);
        assert_from_str("0.1", 100);
        assert_from_str("1.1", 1100);
        refute_from_str("999999999999999999999999999999999");
        refute_from_str("99999999999999999999999999999.999");
        assert_from_str(".99999999999999999999999999999999", 999);
    }

    fn assert_display(milli: u64, expected: &str) {
        let target = Milli::from_millis(milli);

        let actual = target.to_string();

        assert_eq!(actual, expected, "Invalid result for milli {}", milli);
    }

    #[test]
    fn display() {
        assert_display(0, "0.000");
        assert_display(1, "0.001");
        assert_display(100, "0.100");
        assert_display(1000, "1.000");
        assert_display(1001, "1.001");
    }

    quickcheck! {
        fn milli_print_parse_cycle(milli: Milli) -> TestResult {
            let result = milli.to_string().parse();

            assert_eq!(result, Ok(milli));
            TestResult::passed()
        }
    }

    impl Arbitrary for Milli {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            Milli::from_millis(u64::arbitrary(g))
        }
    }
}
