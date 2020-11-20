use crate::scenario::repository::{ScenarioStatus, ScenarioSuiteResult};

use json::object;

pub struct Reporter {
    scenario_suite_result: ScenarioSuiteResult,
}

impl Reporter {
    pub fn new(scenario_suite_result: ScenarioSuiteResult) -> Self {
        Self {
            scenario_suite_result,
        }
    }

    pub fn print(&self) {
        println!(
            "{}",
            object! {
                type: "suite",
                event: "started",
                test_count: self.scenario_suite_result.results().len()
            }
        );

        for scenario in self.scenario_suite_result.results().iter() {
            println!(
                "{}",
                object! {
                    type: "test",
                    event: "started",
                    name: scenario.name()
                }
            );

            match scenario.scenario_status() {
                ScenarioStatus::Passed => {
                    println!(
                        "{}",
                        object! {
                            type: "test",
                            name: scenario.name(),
                            event: "ok"
                        }
                    );
                }
                ScenarioStatus::Ignored => {
                    println!(
                        "{}",
                        object! {
                            type: "test",
                            name: scenario.name(),
                            event: "ignored"
                        }
                    );
                }
                ScenarioStatus::Failed(reason) => {
                    println!(
                        "{}",
                        object! {
                            type: "test",
                            name: scenario.name(),
                            event: "failed",
                            stdout: reason.to_string()
                        }
                    );
                }
            }
        }

        if self.scenario_suite_result.passed() {
            println!(
                "{}",
                object! {
                    type: "suite",
                    event: "ok",
                    passed: self.scenario_suite_result.count_passed(),
                    failed: self.scenario_suite_result.count_failed(),
                    allowed_fail: 0,
                    ignored: self.scenario_suite_result.count_ignored(),
                    measured: 0,
                    filtered_out: 0,
                }
            );
        } else {
            println!(
                "{}",
                object! {
                    type: "suite",
                    event: "failed",
                    passed: self.scenario_suite_result.count_passed(),
                    failed: self.scenario_suite_result.count_failed(),
                    allowed_fail: 0,
                    ignored: self.scenario_suite_result.count_ignored(),
                    measured: 0,
                    filtered_out: 0
                }
            );
        }
    }
}
