use crate::{scenario::repository::ScenarioStatus, test::Result};
use std::{any::Any, fmt};

#[derive(Clone, Debug)]
pub struct ScenarioResult {
    pub name: String,
    pub scenario_status: ScenarioStatus,
}

impl ScenarioResult {
    pub fn passed<S: Into<String>>(name: S) -> Self {
        ScenarioResult {
            name: name.into(),
            scenario_status: ScenarioStatus::Passed,
        }
    }

    pub fn failed<P: Into<String>, S: Into<String>>(name: P, reason: S) -> Self {
        ScenarioResult {
            name: name.into(),
            scenario_status: ScenarioStatus::Failed(reason.into()),
        }
    }

    pub fn ignored<S: Into<String>>(name: S) -> Self {
        ScenarioResult {
            name: name.into(),
            scenario_status: ScenarioStatus::Ignored,
        }
    }

    pub fn scenario_status(&self) -> &ScenarioStatus {
        &self.scenario_status
    }

    pub fn is_failed(&self) -> bool {
        matches!(*self.scenario_status(), ScenarioStatus::Failed { .. })
    }

    pub fn is_ignored(&self) -> bool {
        matches!(*self.scenario_status(), ScenarioStatus::Ignored)
    }

    pub fn is_passed(&self) -> bool {
        matches!(*self.scenario_status(), ScenarioStatus::Passed)
    }

    pub fn name(&self) -> String {
        self.name.to_string()
    }

    pub fn from_result<S: Into<String>>(
        name: S,
        result: std::result::Result<Result<ScenarioResult>, std::boxed::Box<dyn Any + Send>>,
    ) -> ScenarioResult {
        match result {
            Ok(inner) => match inner {
                Ok(scenario_result) => scenario_result,
                Err(err) => ScenarioResult::failed(name, err.to_string()),
            },
            Err(_) => ScenarioResult::failed(name, "no data"),
        }
    }
}

impl fmt::Display for ScenarioResult {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.scenario_status() {
            ScenarioStatus::Passed => write!(f, "passed"),
            ScenarioStatus::Ignored => write!(f, "ignored"),
            ScenarioStatus::Failed(reason) => write!(f, "failed, due to '{}'", &reason),
        }
    }
}
