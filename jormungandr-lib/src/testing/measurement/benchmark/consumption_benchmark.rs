use crate::testing::measurement::{
    attribute::Consumption, marker::ResourcesUsage, thresholds::Thresholds,
};
use std::fmt;
use sysinfo::{ProcessExt, SystemExt};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConsumptionBenchmarkError {
    #[error("couldn't find process with id {0}")]
    NoProcessWitId(usize),
}

#[derive(Clone)]
pub struct ConsumptionBenchmarkDef {
    name: String,
    thresholds: Option<Thresholds<Consumption>>,
    pid: usize,
}

impl ConsumptionBenchmarkDef {
    pub fn new(name: String) -> Self {
        ConsumptionBenchmarkDef {
            name: name,
            pid: 0,
            thresholds: None,
        }
    }

    pub fn name(&self) -> String {
        self.name.clone()
    }

    pub fn bare_metal_stake_pool_consumption_target(&mut self) -> &mut Self {
        self.thresholds = Some(Thresholds::<Consumption>::new_consumption(
            ResourcesUsage::new(10, 50_000, 200_000),
        ));
        self
    }

    pub fn no_target(&mut self) -> &mut Self {
        self.thresholds = None;
        self
    }

    pub fn for_process(&mut self, pid: usize) -> &mut Self {
        self.pid = pid;
        self
    }

    pub fn thresholds(&self) -> Option<&Thresholds<Consumption>> {
        self.thresholds.as_ref()
    }

    pub fn start(&self) -> ConsumptionBenchmarkRun {
        ConsumptionBenchmarkRun {
            definition: self.clone(),
            markers: Vec::new(),
        }
    }
}

pub struct ConsumptionBenchmarkRun {
    definition: ConsumptionBenchmarkDef,
    markers: Vec<ResourcesUsage>,
}

impl ConsumptionBenchmarkRun {
    pub fn snapshot(&mut self) -> Result<(), ConsumptionBenchmarkError> {
        let mut system = sysinfo::System::new_all();
        system.refresh_all();

        let (_, process) = system
            .get_processes()
            .iter()
            .find(|(pid, _)| **pid == self.definition.pid)
            .ok_or(ConsumptionBenchmarkError::NoProcessWitId(
                self.definition.pid,
            ))?;

        let marker = ResourcesUsage::new(
            process.cpu_usage() as u32,
            process.memory() as u32,
            process.virtual_memory() as u32,
        );

        self.markers.push(marker);
        Ok(())
    }

    pub fn exception(self, info: String) -> ConsumptionBenchmarkFinish {
        println!("Test finished prematurely, due to: {}", info);
        self.stop()
    }

    pub fn stop(self) -> ConsumptionBenchmarkFinish {
        match self.definition.thresholds() {
            Some(_thresholds) => ConsumptionBenchmarkFinish {
                definition: self.definition.clone(),
                consumption: Consumption::new(self.markers),
            },
            None => ConsumptionBenchmarkFinish {
                definition: self.definition.clone(),
                consumption: Consumption::new(self.markers),
            },
        }
    }
}

pub struct ConsumptionBenchmarkFinish {
    definition: ConsumptionBenchmarkDef,
    consumption: Consumption,
}

impl ConsumptionBenchmarkFinish {
    pub fn print(&self) {
        println!("{}", &self);
    }
}

impl fmt::Display for ConsumptionBenchmarkFinish {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.definition.thresholds() {
            Some(thresholds) => write!(
                f,
                "Measurement: {}. Result: {}. Actual: {} Thresholds: {}",
                self.definition.name(),
                self.consumption.against(&thresholds),
                self.consumption,
                thresholds,
            ),
            None => write!(
                f,
                "Measurement: {}. Value: {}",
                self.definition.name(),
                self.consumption
            ),
        }
    }
}
