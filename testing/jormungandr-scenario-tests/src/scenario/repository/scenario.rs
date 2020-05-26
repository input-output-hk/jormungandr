use crate::scenario::repository::{ScenarioMethod, Tag};
use std::fmt;

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
            method,
            tags,
        }
    }

    pub fn has_tag(&self, tag: Tag) -> bool {
        self.tags.iter().any(|t| *t == tag)
    }

    pub fn no_tag(&self, tag: Tag) -> bool {
        !self.has_tag(tag)
    }

    pub fn name(&self) -> String {
        self.name.clone()
    }

    pub fn method(&self) -> &ScenarioMethod {
        &self.method
    }
}

impl fmt::Display for Scenario {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self.name())
    }
}
