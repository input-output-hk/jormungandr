use crate::scenario::repository::ScenarioResult;

#[derive(Default, Clone, Debug)]
pub struct ScenarioSuiteResult {
    results: Vec<ScenarioResult>,
}

impl ScenarioSuiteResult {
    pub fn new() -> Self {
        ScenarioSuiteResult {
            results: Vec::new(),
        }
    }

    pub fn push(&mut self, result: ScenarioResult) {
        self.results.push(result)
    }

    pub fn is_failed(&self) -> bool {
        self.count_failed() > 0
    }

    pub fn count_passed(&self) -> usize {
        self.results.iter().filter(|x| x.is_passed()).count()
    }

    pub fn count_failed(&self) -> usize {
        self.results.iter().filter(|x| x.is_failed()).count()
    }

    pub fn count_ignored(&self) -> usize {
        self.results.iter().filter(|x| x.is_ignored()).count()
    }

    fn result_as_string(&self) -> &'static str {
        if self.is_failed() {
            "failed"
        } else {
            "ok"
        }
    }

    pub fn result_string(&self) -> String {
        format!(
            "test result: {}. {} passed; {} failed; {} ignored; 0 measured; 0 filtered out",
            self.result_as_string(),
            self.count_passed(),
            self.count_failed(),
            self.count_ignored()
        )
    }

    pub fn from_single(result: ScenarioResult) -> Self {
        let mut suite_result = Self::new();
        suite_result.push(result);
        suite_result
    }
}
