use crate::testing::measurement::{
    attribute::Endurance, marker::Timestamp, thresholds::Thresholds,
};

use std::{
    fmt,
    time::{Duration, SystemTime},
};
#[derive(Clone)]
pub struct EnduranceBenchmarkDef {
    name: String,
    thresholds: Option<Thresholds<Endurance>>,
}

impl EnduranceBenchmarkDef {
    pub fn new(name: String) -> Self {
        EnduranceBenchmarkDef {
            name,
            thresholds: None,
        }
    }

    pub fn name(&self) -> String {
        self.name.clone()
    }

    pub fn target(&mut self, duration: Duration) -> &mut Self {
        self.thresholds = Some(Thresholds::<Endurance>::new_endurance(duration));
        self
    }

    pub fn no_target(&mut self) -> &mut Self {
        self.thresholds = None;
        self
    }

    pub fn thresholds(&self) -> Option<&Thresholds<Endurance>> {
        self.thresholds.as_ref()
    }

    pub fn start(&self) -> EnduranceBenchmarkRun {
        EnduranceBenchmarkRun {
            definition: self.clone(),
            start_marker: Timestamp::from(SystemTime::now()),
        }
    }
}

pub struct EnduranceBenchmarkRun {
    definition: EnduranceBenchmarkDef,
    start_marker: Timestamp,
}

impl EnduranceBenchmarkRun {
    pub fn stop(&self) -> EnduranceBenchmarkFinish {
        let stop_marker = Timestamp::from(SystemTime::now());
        EnduranceBenchmarkFinish {
            definition: self.definition.clone(),
            endurance: Endurance::new(&self.start_marker, &stop_marker),
        }
    }

    pub fn exception(&self, info: String) -> EnduranceBenchmarkFinish {
        println!("Test finished prematurely, due to: {}", info);
        self.stop()
    }

    pub fn max_endurance_reached(&self) -> bool {
        if let Some(thresholds) = &self.definition.thresholds {
            self.start_marker.elapsed() > thresholds.max().into()
        } else {
            false
        }
    }
}

#[derive(Clone)]
pub struct EnduranceBenchmarkFinish {
    definition: EnduranceBenchmarkDef,
    endurance: Endurance,
}

impl EnduranceBenchmarkFinish {
    pub fn print(&self) {
        println!("{}", &self);
    }

    pub fn print_with_thresholds(&self, thresholds: Thresholds<Endurance>) {
        let mut benchmark_finish = self.clone();
        benchmark_finish.definition.thresholds = Some(thresholds);
        benchmark_finish.print()
    }
}

impl fmt::Display for EnduranceBenchmarkFinish {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.definition.thresholds() {
            Some(thresholds) => write!(
                f,
                "Measurement: {}. Result: {}. Actual: {} Thresholds: {}",
                self.definition.name(),
                self.endurance.against(&thresholds),
                self.endurance,
                thresholds,
            ),
            None => write!(
                f,
                "Measurement: {}. Value: {}",
                self.definition.name(),
                self.endurance
            ),
        }
    }
}
