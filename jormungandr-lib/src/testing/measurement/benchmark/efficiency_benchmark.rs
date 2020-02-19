use crate::testing::measurement::{attribute::Efficiency, marker::Counter, thresholds::Thresholds};

use std::fmt;
#[derive(Clone)]
pub struct EfficiencyBenchmarkDef {
    name: String,
    thresholds: Option<Thresholds<Efficiency>>,
    max: Counter,
}

impl EfficiencyBenchmarkDef {
    pub fn new(name: String) -> Self {
        EfficiencyBenchmarkDef {
            name: name,
            thresholds: None,
            max: 0u32.into(),
        }
    }

    pub fn name(&self) -> String {
        self.name.clone()
    }

    pub fn target(&mut self, target: u32) -> &mut Self {
        self.max = target.into();
        self.thresholds = Some(Thresholds::<Efficiency>::new_efficiency(target));
        self
    }

    pub fn no_target(&mut self) -> &mut Self {
        self.thresholds = None;
        self
    }

    pub fn max(&self) -> Counter {
        self.max
    }

    pub fn thresholds(&self) -> Option<&Thresholds<Efficiency>> {
        self.thresholds.as_ref()
    }

    pub fn start(&self) -> EfficiencyBenchmarkRun {
        EfficiencyBenchmarkRun {
            definition: self.clone(),
            start_marker: Counter::new(),
            current_marker: Counter::new(),
        }
    }
}

pub struct EfficiencyBenchmarkRun {
    definition: EfficiencyBenchmarkDef,
    start_marker: Counter,
    current_marker: Counter,
}

impl EfficiencyBenchmarkRun {
    pub fn increment(&mut self) -> &mut Self {
        self.increment_by(1u32.into())
    }

    pub fn increment_by(&mut self, increment: u32) -> &mut Self {
        let counter: u32 = self.current_marker.into();
        self.current_marker = (counter + increment).into();
        self
    }

    pub fn exception(&self, info: String) -> EfficiencyBenchmarkFinish {
        println!("Test finished prematurely, due to: {}", info);
        self.stop()
    }

    pub fn stop(&self) -> EfficiencyBenchmarkFinish {
        match self.definition.thresholds() {
            Some(_thresholds) => EfficiencyBenchmarkFinish {
                definition: self.definition.clone(),
                efficiency: Efficiency::new(
                    (self.current_marker - self.start_marker).into(),
                    self.definition.max().into(),
                ),
            },
            None => EfficiencyBenchmarkFinish {
                definition: self.definition.clone(),
                efficiency: Efficiency::new(
                    (self.current_marker - self.start_marker).into(),
                    self.current_marker.into(),
                ),
            },
        }
    }
}

pub struct EfficiencyBenchmarkFinish {
    definition: EfficiencyBenchmarkDef,
    efficiency: Efficiency,
}

impl EfficiencyBenchmarkFinish {
    pub fn print(&self) {
        println!("{}", &self);
    }
}

impl fmt::Display for EfficiencyBenchmarkFinish {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.definition.thresholds() {
            Some(thresholds) => write!(
                f,
                "Measurement: {}. Result: {}. Actual: {} Thresholds: {}",
                self.definition.name(),
                self.efficiency.against(&thresholds),
                self.efficiency,
                thresholds,
            ),
            None => write!(
                f,
                "Measurement: {}. Value: {}",
                self.definition.name(),
                self.efficiency
            ),
        }
    }
}
