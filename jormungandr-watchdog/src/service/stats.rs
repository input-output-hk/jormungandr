use atomig::Atomic;
use std::sync::atomic::{AtomicU64, Ordering};

/// keep some stats based on [Welford's online algorithm]
///
/// [Welford's online algorithm]: https://en.wikipedia.org/wiki/Algorithms_for_calculating_variance#Welford's_online_algorithm
pub struct Stats {
    count: AtomicU64,
    mean: Atomic<f64>,
    m2: Atomic<f64>,
}

impl Stats {
    pub fn new() -> Self {
        Self {
            count: AtomicU64::new(0),
            mean: Atomic::new(0.0),
            m2: Atomic::new(0.0),
        }
    }

    pub fn push(&self, entry: f64) {
        self.count.fetch_add(1, Ordering::Relaxed);
        let self_count = self.count.load(Ordering::Relaxed);
        if self_count == 1 {
            self.mean.store(entry, Ordering::Relaxed);
            self.m2.store(0.0, Ordering::Relaxed);
        } else {
            let delta = entry - self.mean.load(Ordering::Relaxed);
            self.mean.store(
                self.mean.load(Ordering::Acquire) + delta / self_count as f64,
                Ordering::Release,
            );
            let delta2 = entry - self.mean.load(Ordering::Relaxed);
            self.m2.store(
                self.m2.load(Ordering::Acquire) + delta * delta2,
                Ordering::Release,
            )
        }
    }

    pub fn count(&self) -> usize {
        self.count.load(Ordering::Relaxed) as usize
    }

    pub fn mean(&self) -> f64 {
        self.mean.load(Ordering::Relaxed)
    }

    pub fn variance(&self) -> f64 {
        if self.count() < 2 {
            0.0
        } else {
            self.m2.load(Ordering::Relaxed) / (self.count.load(Ordering::Relaxed) - 1) as f64
        }
    }

    pub fn standard_derivation(&self) -> f64 {
        self.m2.load(Ordering::Relaxed).sqrt()
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
