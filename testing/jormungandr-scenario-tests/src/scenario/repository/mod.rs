mod result;
mod scenario;
mod suite_result;
mod tag;

pub use result::ScenarioResult;
pub use scenario::Scenario;
pub use suite_result::ScenarioSuiteResult;
pub use tag::{parse_tag_from_str, Tag};

use crate::{
    test::{
        comm::leader_leader::*,
        comm::passive_leader::*,
        features::{leader_promotion::*, p2p::*},
        network::real::real_network,
        network::topology::scenarios::*,
        non_functional::{disruption::*, legacy, soak::*},
        Result,
    },
    Context,
};

use rand_chacha::ChaChaRng;
type ScenarioMethod = fn(Context<ChaChaRng>) -> Result<ScenarioResult>;

pub struct ScenariosRepository {
    repository: Vec<Scenario>,
    scenario: String,
    tag: Tag,
}

impl ScenariosRepository {
    pub fn new<S: Into<String>>(scenario: S, tag: Tag) -> Self {
        Self {
            repository: scenarios_repository(),
            scenario: scenario.into(),
            tag,
        }
    }

    pub fn run(&self, context: &Context<ChaChaRng>) -> ScenarioSuiteResult {
        let available_scenarios = self.scenarios_tagged_by(self.tag);

        if self.should_run_all() {
            self.run_all_scenarios(&available_scenarios, &mut context.clone())
        } else {
            ScenarioSuiteResult::from_single(self.run_single_scenario(
                &self.scenario,
                &available_scenarios,
                &mut context.clone(),
            ))
        }
    }

    fn scenarios_tagged_by(&self, tag: Tag) -> Vec<Scenario> {
        match tag {
            Tag::All => self.repository.clone(),
            Tag::Unstable => self
                .repository
                .iter()
                .cloned()
                .filter(|x| x.has_tag(tag))
                .collect(),
            _ => self
                .repository
                .iter()
                .cloned()
                .filter(|x| x.has_tag(tag) && x.no_tag(Tag::Unstable))
                .collect(),
        }
    }

    fn should_run_all(&self) -> bool {
        self.scenario.trim() == "*"
    }

    fn run_all_scenarios(
        &self,
        available_scenarios: &[Scenario],
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
        scenarios_to_run: &[Scenario],
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
        let scenario_to_run = scenario.method();

        println!("Running '{}' scenario", scenario.name());

        let result = std::panic::catch_unwind(|| scenario_to_run(context.clone().derive()));
        let scenario_result = ScenarioResult::from_result(result);
        println!("Scenario '{}' {}", scenario.name(), scenario_result);
        scenario_result
    }
}

#[derive(Clone, Debug)]
pub enum ScenarioStatus {
    Passed,
    Failed(String),
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
        vec![Tag::Short, Tag::Unstable],
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
    repository.push(Scenario::new(
        "passive_leader_disruption_no_overlap",
        passive_leader_disruption_no_overlap,
        vec![Tag::Short],
    ));
    repository.push(Scenario::new(
        "passive_leader_disruption_overlap",
        passive_leader_disruption_overlap,
        vec![Tag::Short],
    ));
    repository.push(Scenario::new(
        "leader_leader_disruption_overlap",
        leader_leader_disruption_overlap,
        vec![Tag::Short],
    ));
    repository.push(Scenario::new(
        "leader_leader_disruption_no_overlap",
        leader_leader_disruption_no_overlap,
        vec![Tag::Short],
    ));
    repository.push(Scenario::new(
        "point_to_point_disruption",
        point_to_point_disruption,
        vec![Tag::Short],
    ));
    repository.push(Scenario::new(
        "custom_network_disruption",
        custom_network_disruption,
        vec![Tag::Short],
    ));

    repository.push(Scenario::new(
        "passive_node_promotion",
        passive_node_promotion,
        vec![Tag::Short],
    ));

    // TODO: Try to make ScenarioMethod a boxed closure
    // so that we could do this:
    //  for n in (1..=5).rev() {
    //      repository.push(Scenario::new(
    //          legacy::last_nth_release_title(n),
    //          |ctx| legacy::last_nth_release(ctx, n),
    //          vec![Tag::Short],
    //      ));
    //  }

    repository.push(Scenario::new(
        legacy::last_nth_release_title(5),
        |ctx| legacy::last_nth_release(ctx, 5),
        vec![Tag::Short],
    ));

    repository.push(Scenario::new(
        legacy::last_nth_release_title(4),
        |ctx| legacy::last_nth_release(ctx, 4),
        vec![Tag::Short],
    ));

    repository.push(Scenario::new(
        legacy::last_nth_release_title(3),
        |ctx| legacy::last_nth_release(ctx, 3),
        vec![Tag::Short],
    ));

    repository.push(Scenario::new(
        legacy::last_nth_release_title(2),
        |ctx| legacy::last_nth_release(ctx, 2),
        vec![Tag::Short],
    ));

    repository.push(Scenario::new(
        legacy::last_nth_release_title(1),
        |ctx| legacy::last_nth_release(ctx, 1),
        vec![Tag::Short],
    ));

    repository.push(Scenario::new(
        legacy::disruption_last_nth_release_title(5),
        |ctx| legacy::disruption_last_nth_release(ctx, 5),
        vec![Tag::Short, Tag::Unstable],
    ));

    repository.push(Scenario::new(
        legacy::disruption_last_nth_release_title(4),
        |ctx| legacy::disruption_last_nth_release(ctx, 4),
        vec![Tag::Short, Tag::Unstable],
    ));

    repository.push(Scenario::new(
        legacy::disruption_last_nth_release_title(3),
        |ctx| legacy::disruption_last_nth_release(ctx, 3),
        vec![Tag::Short, Tag::Unstable],
    ));

    repository.push(Scenario::new(
        legacy::disruption_last_nth_release_title(2),
        |ctx| legacy::disruption_last_nth_release(ctx, 2),
        vec![Tag::Short, Tag::Unstable],
    ));

    repository.push(Scenario::new(
        legacy::disruption_last_nth_release_title(1),
        |ctx| legacy::disruption_last_nth_release(ctx, 1),
        vec![Tag::Short, Tag::Unstable],
    ));

    repository.push(Scenario::new("relay_soak", relay_soak, vec![Tag::Long]));

    repository.push(Scenario::new(
        "p2p_stats_test",
        p2p_stats_test,
        vec![Tag::Short],
    ));

    repository.push(Scenario::new(
        "max_connections",
        max_connections,
        vec![Tag::Short],
    ));

    repository.push(Scenario::new("real_network", real_network, vec![Tag::Long]));
    repository.push(Scenario::new(
        "mesh_disruption",
        mesh_disruption,
        vec![Tag::Short],
    ));
    repository
}
