pub struct Stats {
    count: u64,
    mean: f64,
    m2: f64,
}

impl Stats {
    pub fn new() -> Self {
        Self {
            count: 0,
            mean: 0.0,
            m2: 0.0,
        }
    }

    pub fn push(&mut self, entry: f64) {
        self.count += 1;

        if self.count == 1 {
            self.mean = entry;
            self.m2 = 0.0;
        } else {
            let delta = entry - self.mean;
            self.mean += delta / self.count as f64;
            let delta2 = entry - self.mean;
            self.m2 += delta * delta2;
        }
    }

    pub fn count(&self) -> usize {
        self.count as usize
    }

    pub fn mean(&self) -> f64 {
        self.mean
    }

    pub fn variance(&self) -> f64 {
        if self.count() < 2 {
            0.0
        } else {
            self.m2 / (self.count - 1) as f64
        }
    }

    pub fn standard_derivation(&self) -> f64 {
        self.m2.sqrt()
    }
}

impl Default for Stats {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_entries() {
        let stats = Stats::new();

        assert_eq!(stats.count(), 0);
        approx::assert_relative_eq!(stats.mean(), 0.0);
        approx::assert_relative_eq!(stats.variance(), 0.0);
        approx::assert_relative_eq!(stats.standard_derivation(), 0.0);
    }

    #[test]
    fn one_entry() {
        let mut stats = Stats::new();

        stats.push(10.0);

        assert_eq!(stats.count(), 1);
        approx::assert_relative_eq!(stats.mean(), 10.0);
        approx::assert_relative_eq!(stats.variance(), 0.0);
        approx::assert_relative_eq!(stats.standard_derivation(), 0.0);
    }

    #[test]
    fn tow_entries() {
        let mut stats = Stats::new();

        stats.push(10.0);
        stats.push(20.0);

        assert_eq!(stats.count(), 2);
        approx::assert_relative_eq!(stats.mean(), 15.0);
        approx::assert_relative_eq!(stats.variance(), 50.0);
        approx::assert_relative_eq!(stats.standard_derivation(), 7.07, max_relative = 0.001,);
    }

    fn unit(entries: &[f64], mean: f64, variance: f64, standard_derivation: f64) {
        let count = entries.len();

        let mut stats = Stats::new();

        for i in entries {
            stats.push(*i);
        }

        assert_eq!(stats.count(), count);
        approx::assert_relative_eq!(stats.mean(), mean, max_relative = 0.001);
        approx::assert_relative_eq!(stats.variance(), variance, max_relative = 0.001);
        approx::assert_relative_eq!(
            stats.standard_derivation(),
            standard_derivation,
            max_relative = 0.001,
        );
    }

    #[test]
    fn golden() {
        unit(&[15.0, 15.0], 15.0, 0.0, 0.0);
        unit(&[14.0, 16.0], 15.0, 2.0, 1.414);
        unit(&[10.0, 15.0, 20.0], 15.0, 25.0, 7.071);
        unit(&[10.0, 11.0, 15.0, 19.0, 20.0], 15.0, 20.5, 9.055);
    }
}
