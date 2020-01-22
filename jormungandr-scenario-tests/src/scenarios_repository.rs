use crate::{
    test::Result,
    test::{
        comm::leader_leader::*,
        comm::passive_leader::*,
        network::topology::scenarios::*,
        non_functional::{disruption::*, soak::*},
    },
    Context,
};
use rand_chacha::ChaChaRng;
use std::{any::Any, fmt, marker::Send};
type ScenarioMethod = fn(Context<ChaChaRng>) -> Result<ScenarioResult>;

pub struct ScenariosRepository {
    repository: Vec<Scenario>,
    scenario: String,
    tag: Tag,
}

#[derive(Debug, Clone)]
pub struct Scenario {
    name: String,
    method: ScenarioMethod,
    tags: Vec<Tag>,
}

impl Scenario {
    pub fn new<S: Into<String>>(name: S, method: ScenarioMethod, tags: Vec<Tag>) -> Self {
        Scenario {
            name: name.into(),
            method: method,
            tags: tags,
        }
    }

    pub fn has_tag(&self, tag: &Tag) -> bool {
        self.tags.iter().any(|t| *t == *tag)
    }

    pub fn name(&self) -> String {
        self.name.clone()
    }
}

impl fmt::Display for Scenario {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self.name())
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Tag {
    Short,
    Long,
    All,
}

pub fn parse_tag_from_str(tag: &str) -> Result<Tag> {
    let tag_lowercase: &str = &tag.to_lowercase();
    match tag_lowercase {
        "short" => Ok(Tag::Short),
        "long" => Ok(Tag::Long),
        _ => Ok(Tag::All),
    }
}

impl fmt::Display for Tag {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl ScenariosRepository {
    pub fn new<S: Into<String>>(scenario: S, tag: Tag) -> Self {
        Self {
            repository: scenarios_repository(),
            scenario: scenario.into(),
            tag: tag,
        }
    }

    pub fn run(&self, context: &Context<ChaChaRng>) -> ScenarioSuiteResult {
        let available_scenarios = self.scenarios_tagged_by(&self.tag);

        match self.should_run_all() {
            true => self.run_all_scenarios(&available_scenarios, &mut context.clone()),
            false => ScenarioSuiteResult::from_single(self.run_single_scenario(
                &self.scenario,
                &available_scenarios,
                &mut context.clone(),
            )),
        }
    }

    fn scenarios_tagged_by(&self, tag: &Tag) -> Vec<Scenario> {
        match tag {
            Tag::All => self.repository.clone(),
            _ => self
                .repository
                .iter()
                .cloned()
                .filter(|x| x.has_tag(tag))
                .collect(),
        }
    }

    fn should_run_all(&self) -> bool {
        self.scenario.trim() == "*"
    }

    fn run_all_scenarios(
        &self,
        available_scenarios: &Vec<Scenario>,
        mut context: &mut Context<ChaChaRng>,
    ) -> ScenarioSuiteResult {
        let mut suite_result = ScenarioSuiteResult::new();
        for scenario_to_run in available_scenarios {
            suite_result.push(self.run_single_scenario(
                &scenario_to_run.name(),
                &available_scenarios,
                &mut context,
            ));
        }
        suite_result
    }

    fn run_single_scenario(
        &self,
        scenario_name: &str,
        scenarios_to_run: &Vec<Scenario>,
        context: &mut Context<ChaChaRng>,
    ) -> ScenarioResult {
        let scenario = self
            .repository
            .iter()
            .find(|x| x.name() == scenario_name)
            .unwrap_or_else(|| {
                panic!(format!(
                    "Cannot find scenario '{}' under the tag '{:?}'. Available are: {:?}",
                    scenario_name, self.tag, scenarios_to_run
                ))
            });
        let scenario_to_run = scenario.method;

        println!("Running '{}' scenario", scenario.name());
        let result = std::panic::catch_unwind(|| return scenario_to_run(context.clone().derive()));
        let scenario_result = ScenarioResult::from_result(result);
        println!("Scenario '{}' {}", scenario.name(), scenario_result);
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
fn scenarios_repository() -> Vec<Scenario> {
    let mut repository: Vec<Scenario> = Vec::new();
    repository.push(Scenario::new(
        "two_transaction_to_two_leaders",
        two_transaction_to_two_leaders,
        vec![Tag::Short],
    ));
    repository.push(Scenario::new(
        "transaction_to_passive",
        transaction_to_passive,
        vec![Tag::Short],
    ));
    repository.push(Scenario::new(
        "leader_restart",
        leader_restart,
        vec![Tag::Short],
    ));
    repository.push(Scenario::new(
        "passive_node_is_updated",
        passive_node_is_updated,
        vec![Tag::Short],
    ));
    repository.push(Scenario::new(
        "fully_connected",
        fully_connected,
        vec![Tag::Short],
    ));
    repository.push(Scenario::new("star", star, vec![Tag::Short]));
    repository.push(Scenario::new("ring", ring, vec![Tag::Short]));
    repository.push(Scenario::new("mesh", mesh, vec![Tag::Short]));
    repository.push(Scenario::new(
        "point_to_point",
        point_to_point,
        vec![Tag::Short],
    ));
    repository.push(Scenario::new(
        "point_to_point_on_file_storage",
        point_to_point_on_file_storage,
        vec![Tag::Short],
    ));
    repository.push(Scenario::new("tree", tree, vec![Tag::Short]));
    repository.push(Scenario::new("relay", relay, vec![Tag::Short]));
    repository.push(Scenario::new("relay_soak", relay_soak, vec![Tag::Long]));
    repository.push(Scenario::new(
        "mesh_disruption",
        mesh_disruption,
        vec![Tag::Short],
    ));
    repository
}
