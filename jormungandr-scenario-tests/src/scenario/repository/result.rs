use crate::{
    scenario::repository::{Measurement, ScenarioStatus},
    test::Result,
};
use std::{any::Any, fmt};

#[derive(Clone, Debug)]
pub struct ScenarioResult {
    pub scenario_status: ScenarioStatus,
    pub measurements: Vec<Measurement>,
}

impl ScenarioResult {
    pub fn passed() -> Self {
        ScenarioResult {
            scenario_status: ScenarioStatus::Passed,
            measurements: vec![],
        }
    }

    pub fn passed_with_measurements(measurements: Vec<Measurement>) -> Self {
        ScenarioResult {
            scenario_status: ScenarioStatus::Passed,
            measurements: measurements,
        }
    }

    pub fn failed<S: Into<String>>(reason: S) -> Self {
        ScenarioResult {
            scenario_status: ScenarioStatus::Failed(reason.into()),
            measurements: vec![],
        }
    }

    pub fn scenario_status(&self) -> &ScenarioStatus {
        &self.scenario_status
    }

    pub fn add_measurement(&mut self, measurement: Measurement) {
        self.measurements.push(measurement);
    }

    pub fn measurements(&self) -> Vec<Measurement> {
        self.measurements.clone()
    }

    pub fn is_failed(&self) -> bool {
        match *self.scenario_status() {
            ScenarioStatus::Passed => false,
            ScenarioStatus::Failed { .. } => true,
        }
    }

    pub fn from_result(
        result: std::result::Result<Result<ScenarioResult>, std::boxed::Box<dyn Any + Send>>,
    ) -> ScenarioResult {
        match result {
            Ok(inner) => match inner {
                Ok(scenario_result) => scenario_result,
                Err(err) => ScenarioResult::failed(err.to_string()),
            },
            Err(_) => ScenarioResult::failed("no data".to_string()),
        }
    }
}

impl fmt::Display for ScenarioResult {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.scenario_status() {
            ScenarioStatus::Passed => write!(f, "passed"),
            ScenarioStatus::Failed(reason) => write!(f, "failed, due to '{}'", &reason),
        }
    }
}
