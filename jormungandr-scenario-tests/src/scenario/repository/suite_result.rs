use crate::scenario::repository::{Measurement, ScenarioResult};

#[derive(Clone, Debug)]
pub struct ScenarioSuiteResult {
    results: Vec<ScenarioResult>,
}

impl ScenarioSuiteResult {
    pub fn new() -> Self {
        ScenarioSuiteResult {
            results: Vec::new(),
        }
    }

    pub fn any_measurements(&self) -> bool {
        self.results.iter().any(|x| !x.measurements().is_empty())
    }

    pub fn print_measurements_results(&self) {
        let mut measurements: Vec<Measurement> = Vec::new();
        for scenario_result in self.results.iter() {
            measurements.extend(scenario_result.measurements.iter().cloned());
        }

        for measurement in measurements {
            println!("{}", measurement);
        }
    }

    pub fn push(&mut self, result: ScenarioResult) {
        self.results.push(result)
    }

    pub fn is_failed(&self) -> bool {
        self.results.iter().any(|x| x.is_failed())
    }

    pub fn count_passed(&self) -> usize {
        self.results.iter().filter(|x| !x.is_failed()).count()
    }

    pub fn count_failed(&self) -> usize {
        self.results.iter().filter(|x| x.is_failed()).count()
    }

    fn result_as_string(&self) -> String {
        match self.is_failed() {
            true => "failed".to_owned(),
            false => "ok".to_owned(),
        }
    }

    pub fn result_string(&self) -> String {
        format!(
            "test result: {}. {} passed; {} failed; 0 ignored; 0 measured; 0 filtered out",
            self.result_as_string(),
            self.count_passed(),
            self.count_failed()
        )
    }

    pub fn from_single(result: ScenarioResult) -> Self {
        let mut suite_result = Self::new();
        suite_result.push(result);
        suite_result
    }
}
