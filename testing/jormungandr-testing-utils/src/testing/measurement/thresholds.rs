use super::{
    attribute::{Consumption, Efficiency, Endurance, Speed},
    marker::ResourcesUsage,
    Status,
};

use std::{cmp::PartialOrd, fmt, time::Duration};

#[derive(Clone, Debug)]
pub struct Thresholds<T> {
    inner_thresholds: Vec<(Status, T)>,
    max: T,
}

impl<T: PartialOrd + Clone> Thresholds<T> {
    pub fn new(green: T, yellow: T, red: T, max: T) -> Self {
        Self {
            inner_thresholds: vec![
                (Status::Green, green),
                (Status::Yellow, yellow),
                (Status::Red, red),
            ],
            max,
        }
    }

    pub fn thresholds(&self) -> &Vec<(Status, T)> {
        &self.inner_thresholds
    }

    pub fn max(&self) -> T {
        self.max.clone()
    }

    pub fn green_threshold(&self) -> T {
        self.thresholds()
            .iter()
            .find(|(x, _)| *x == Status::Green)
            .expect("cannot find green threshold")
            .1
            .clone()
    }

    pub fn yellow_threshold(&self) -> T {
        self.thresholds()
            .iter()
            .find(|(x, _)| *x == Status::Yellow)
            .expect("cannot find green threshold")
            .1
            .clone()
    }

    pub fn red_threshold(&self) -> T {
        self.thresholds()
            .iter()
            .find(|(x, _)| *x == Status::Red)
            .expect("cannot find red threshold")
            .1
            .clone()
    }
}

impl<T: fmt::Display + Clone + fmt::Debug + std::cmp::PartialOrd> fmt::Display for Thresholds<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Green: {} Yellow: {} Red: {} Max: {}",
            self.green_threshold(),
            self.yellow_threshold(),
            self.red_threshold(),
            self.max()
        )
    }
}

impl Thresholds<Endurance> {
    pub fn new_endurance(duration: Duration) -> Thresholds<Endurance> {
        let green = Duration::from_secs(duration.as_secs() / 2);
        let yellow = Duration::from_secs(duration.as_secs() / 3);
        let red = Duration::from_secs(duration.as_secs() / 4);
        Thresholds::<Endurance>::new(
            green.into(),
            yellow.into(),
            red.into(),
            Duration::from_secs(duration.as_secs()).into(),
        )
    }
}

impl Thresholds<Speed> {
    pub fn new_speed(duration: Duration) -> Thresholds<Speed> {
        let green = Duration::from_secs(duration.as_secs() / 4);
        let yellow = Duration::from_secs(duration.as_secs() / 3);
        let red = Duration::from_secs(duration.as_secs() / 2);
        Thresholds::<Speed>::new(
            green.into(),
            yellow.into(),
            red.into(),
            Duration::from_secs(duration.as_secs()).into(),
        )
    }
}

impl Thresholds<Efficiency> {
    pub fn new_efficiency(target: u32) -> Thresholds<Efficiency> {
        let green = Efficiency::new(target / 2, target);
        let yellow = Efficiency::new(target / 3, target);
        let red = Efficiency::new(target / 4, target);
        let max = Efficiency::new(target, target);
        Thresholds::<Efficiency>::new(green, yellow, red, max)
    }
}

impl Thresholds<Consumption> {
    pub fn new_consumption(resources_usage: ResourcesUsage) -> Thresholds<Consumption> {
        let target_cpu = resources_usage.cpu_usage();
        let target_memory = resources_usage.memory_usage();
        let target_virtual_memory = resources_usage.virtual_memory_usage();

        let green_marker =
            ResourcesUsage::new(target_cpu / 4, target_memory / 4, target_virtual_memory / 4);
        let green = Consumption::new(vec![green_marker]);

        let yellow_marker =
            ResourcesUsage::new(target_cpu / 3, target_memory / 3, target_virtual_memory / 3);
        let yellow = Consumption::new(vec![yellow_marker]);

        let red_marker =
            ResourcesUsage::new(target_cpu / 2, target_memory / 2, target_virtual_memory / 2);
        let red = Consumption::new(vec![red_marker]);

        let max_marker = ResourcesUsage::new(target_cpu, target_memory, target_virtual_memory);
        let max = Consumption::new(vec![max_marker]);
        Thresholds::<Consumption>::new(green, yellow, red, max)
    }
}
