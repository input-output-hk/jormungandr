use super::stake::PercentStake;

/// number between 0.0 and 1.0 that is used to calculate to
pub struct F(f64);

impl F {
    // TODO: error handling and replace by TryFrom once more stable
    pub fn create(v: f64) -> F {
        assert!(v > 0.0 && v <= 1.0);
        F(v)
    }
}

/// Threshold between 0.0 and 1.0
#[derive(Clone,Copy,PartialEq,PartialOrd)]
pub struct Threshold(f64);

impl Threshold {
    pub fn from_u256(v: &[u8]) -> Self {
        assert_eq!(v.len(), 32);
        // TODO, only consider the highest part
        let v64 = (v[0] as u64) << 56 |
                  (v[1] as u64) << 48 |
                  (v[2] as u64) << 40 |
                  (v[3] as u64) << 32 |
                  (v[4] as u64) << 24 |
                  (v[5] as u64) << 16 |
                  (v[6] as u64) << 8  |
                  (v[7] as u64);
        Threshold((v64 as f64) / 18446744073709551616.0)
    }
}

pub fn phi(f: F, rs: PercentStake) -> Threshold {
    Threshold(1.0 - (1.0 - f.0).powf(rs.0))
}
