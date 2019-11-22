use crate::{
    test::Result,
    test::{comm::leader_leader::*, comm::passive_leader::*, network::topology::scenarios::*},
    Context,
};
use rand_chacha::ChaChaRng;
use std::{any::Any, collections::HashMap, fmt, marker::Send};
type ScenarioMethod = fn(Context<ChaChaRng>) -> Result<ScenarioResult>;

pub struct ScenariosRepository {
    repository: HashMap<String, ScenarioMethod>,
    scenario: String,
}

impl ScenariosRepository {
    pub fn new<S: Into<String>>(scenario: S) -> Self {
        Self {
            repository: scenarios_repository(),
            scenario: scenario.into(),
        }
    }

    pub fn run(&self, context: &Context<ChaChaRng>) -> ScenarioSuiteResult {
        match self.should_run_all() {
            true => self.run_all_scenarios(&mut context.clone()),
            false => ScenarioSuiteResult::from_single(
                self.run_scenario_by_name(&self.scenario, &mut context.clone()),
            ),
        }
    }

    fn should_run_all(&self) -> bool {
        self.scenario.trim() == "*"
    }

    fn run_all_scenarios(&self, mut context: &mut Context<ChaChaRng>) -> ScenarioSuiteResult {
        let mut suite_result = ScenarioSuiteResult::new();
        for scenario_to_run in self.repository.keys() {
            suite_result.push(self.run_scenario_by_name(&scenario_to_run, &mut context));
        }
        suite_result
    }

    fn run_scenario_by_name(
        &self,
        scenario: &str,
        context: &mut Context<ChaChaRng>,
    ) -> ScenarioResult {
        if !self.repository.contains_key(scenario) {
            panic!(format!(
                "Cannot find scenario '{}'. Available are: {:?}",
                scenario,
                self.repository.keys().cloned().collect::<Vec<String>>()
            ));
        }

        let scenario_to_run = self.repository.get(scenario).unwrap();
        println!("Running '{}' scenario", scenario);
        let result = std::panic::catch_unwind(|| return scenario_to_run(context.clone().derive()));
        let scenario_result = ScenarioResult::from_result(result);
        println!("Scenario '{}' {}", scenario, scenario_result);
        scenario_result
    }
}

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

#[derive(Clone, Debug)]
pub enum ScenarioResult {
    Passed,
    Failed(String),
}

impl ScenarioResult {
    pub fn failed<S: Into<String>>(reason: S) -> Self {
        ScenarioResult::Failed(reason.into())
    }

    pub fn is_failed(&self) -> bool {
        match self {
            ScenarioResult::Passed => false,
            ScenarioResult::Failed { .. } => true,
        }
    }

    pub fn from_result(
        result: std::result::Result<Result<ScenarioResult>, std::boxed::Box<dyn Any + Send>>,
    ) -> ScenarioResult {
        match result {
            Ok(inner) => match inner {
                Ok(scenario_result) => scenario_result,
                Err(err) => ScenarioResult::Failed(err.to_string()),
            },
            Err(_) => ScenarioResult::Failed("no data".to_string()),
        }
    }
}

impl fmt::Display for ScenarioResult {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ScenarioResult::Passed => write!(f, "passed"),
            ScenarioResult::Failed(reason) => write!(f, "failed, due to '{}'", reason),
        }
    }
}
fn scenarios_repository() -> HashMap<String, ScenarioMethod> {
    let mut map: HashMap<String, ScenarioMethod> = HashMap::new();
    map.insert(
        "two_transaction_to_two_leaders".to_string(),
        two_transaction_to_two_leaders,
    );
    map.insert("transaction_to_passive".to_string(), transaction_to_passive);
    map.insert("leader_restart".to_string(), leader_restart);
    map.insert(
        "passive_node_is_updated".to_string(),
        passive_node_is_updated,
    );
    map.insert("fully_connected".to_string(), fully_connected);
    map.insert("star".to_string(), star);
    map.insert("ring".to_string(), ring);
    map.insert("mesh".to_string(), mesh);
    map.insert("point_to_point".to_string(), point_to_point);
    map.insert("tree".to_string(), tree);
    map.insert("relay".to_string(), relay);
    map
}
