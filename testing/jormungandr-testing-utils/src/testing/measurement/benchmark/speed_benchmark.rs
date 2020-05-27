use crate::testing::measurement::{attribute::Speed, marker::Timestamp, thresholds::Thresholds};

use std::{
    fmt,
    time::{Duration, SystemTime},
};

#[derive(Clone)]
pub struct SpeedBenchmarkDef {
    name: String,
    thresholds: Option<Thresholds<Speed>>,
}

impl SpeedBenchmarkDef {
    pub fn new(name: String) -> Self {
        SpeedBenchmarkDef {
            name,
            thresholds: None,
        }
    }

    pub fn name(&self) -> String {
        self.name.clone()
    }

    pub fn with_thresholds(&mut self, thresholds: Thresholds<Speed>) -> &mut Self {
        self.thresholds = Some(thresholds);
        self
    }

    pub fn target(&mut self, duration: Duration) -> &mut Self {
        self.with_thresholds(Thresholds::<Speed>::new_speed(duration))
    }

    pub fn thresholds(&self) -> Option<&Thresholds<Speed>> {
        self.thresholds.as_ref()
    }

    pub fn no_target(&mut self) -> &mut Self {
        self.thresholds = None;
        self
    }

    pub fn start(&self) -> SpeedBenchmarkRun {
        SpeedBenchmarkRun {
            definition: self.clone(),
            start_marker: Timestamp::from(SystemTime::now()),
        }
    }
}

pub struct SpeedBenchmarkRun {
    definition: SpeedBenchmarkDef,
    start_marker: Timestamp,
}

impl SpeedBenchmarkRun {
    pub fn stop(&self) -> SpeedBenchmarkFinish {
        let stop_marker = Timestamp::from(SystemTime::now());
        SpeedBenchmarkFinish {
            definition: self.definition.clone(),
            speed: Speed::new(&self.start_marker, &stop_marker),
        }
    }

    pub fn timeout_exceeded(&self) -> bool {
        if let Some(thresholds) = &self.definition.thresholds {
            self.start_marker.elapsed() > thresholds.max().into()
        } else {
            false
        }
    }

    pub fn definition(&self) -> &SpeedBenchmarkDef {
        &self.definition
    }
}

pub struct SpeedBenchmarkFinish {
    definition: SpeedBenchmarkDef,
    speed: Speed,
}

impl SpeedBenchmarkFinish {
    pub fn print(&self) {
        println!("{}", &self);
    }

    pub fn new(definition: SpeedBenchmarkDef, speed: Speed) -> Self {
        Self { definition, speed }
    }
}

impl fmt::Display for SpeedBenchmarkFinish {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.definition.thresholds() {
            Some(thresholds) => write!(
                f,
                "Measurement: {}. Result: {}. Actual: {} Thresholds: {}",
                self.definition.name(),
                self.speed.against(&thresholds),
                self.speed,
                thresholds,
            ),
            None => write!(
                f,
                "Measurement: {}. Value: {}",
                self.definition.name(),
                self.speed
            ),
        }
    }
}
