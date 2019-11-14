use crate::{
    example_scenarios::*,
    test::{comm::leader_leader::*, comm::passive_leader::*, network::topology::scenarios::*},
    Context,
};
use rand_chacha::ChaChaRng;
use std::collections::HashMap;

type ScenarioMethod = fn(Context<ChaChaRng>) -> ();

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

    pub fn run(&self, mut context: &mut Context<ChaChaRng>) {
        match self.should_run_all() {
            true => self.run_all_scenarios(&mut context),
            false => self.run_scenario_by_name(&self.scenario, &mut context),
        }
    }

    fn should_run_all(&self) -> bool {
        self.scenario.trim() == "*"
    }

    fn run_all_scenarios(&self, mut context: &mut Context<ChaChaRng>) {
        for scenario_to_run in scenarios_repository().keys() {
            self.run_scenario_by_name(&scenario_to_run, &mut context);
        }
        RunReporter::after_suite();
    }

    fn run_scenario_by_name(&self, scenario: &str, context: &mut Context<ChaChaRng>) {
        let repo = scenarios_repository();
        if !repo.contains_key(scenario) {
            panic!(format!(
                "Cannot find scenario '{}'. Available are: {:?}",
                scenario,
                repo.keys().cloned().collect::<Vec<String>>()
            ));
        }

        let scenario_to_run = repo.get(scenario).unwrap();
        let run_reporter = RunReporter::new(scenario);

        run_reporter.before_scenario();
        scenario_to_run(context.derive());
        run_reporter.after_scenario();
    }
}

struct RunReporter {
    scenario: String,
}

impl RunReporter {
    pub fn new<S: Into<String>>(scenario: S) -> Self {
        RunReporter {
            scenario: scenario.into(),
        }
    }

    pub fn before_scenario(&self) {
        println!("Running '{}' scenario", self.scenario);
    }

    pub fn after_scenario(&self) {
        println!("Scenario '{}' completed", self.scenario);
    }

    pub fn after_suite() {
        print!("Suite completed");
    }
}

fn scenarios_repository() -> HashMap<String, ScenarioMethod> {
    let mut map: HashMap<String, ScenarioMethod> = HashMap::new();
    map.insert(
        "two_transaction_to_two_leaders".to_string(),
        two_transaction_to_two_leaders,
    );
    map.insert("transaction_to_passive".to_string(), transaction_to_passive);
    map.insert("leader_is_offline".to_string(), leader_is_offline);
    map.insert(
        "leader_is_online_with_delay".to_string(),
        leader_is_online_with_delay,
    );
    map.insert("leader_restart".to_string(), leader_restart);
    map.insert(
        "passive_node_is_updated".to_string(),
        passive_node_is_updated,
    );
    map.insert("star".to_string(), star);
    map.insert("ring".to_string(), ring);
    map.insert("mesh".to_string(), mesh);
    map.insert("point_to_point".to_string(), point_to_point);
    map.insert("tree".to_string(), tree);
    map.insert("relay".to_string(), relay);
    map
}
