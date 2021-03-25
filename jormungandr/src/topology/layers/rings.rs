use crate::topology::topic;
use jormungandr_lib::interfaces::TopicsOfInterest;
use poldercast::{
    layer::{Layer, ViewBuilder},
    InterestLevel, PriorityMap, Profile, Topic,
};
use std::convert::TryFrom;
use thiserror::Error;

const LOW_INTERESET: InterestLevel = InterestLevel::new(1);
const NORMAL_INTEREST: InterestLevel = InterestLevel::new(3);
const HIGH_INTEREST: InterestLevel = InterestLevel::new(5);

#[derive(Clone)]
pub struct RingsConfig {
    messages: InterestLevel,
    blocks: InterestLevel,
}

impl Default for RingsConfig {
    fn default() -> Self {
        Self {
            messages: NORMAL_INTEREST,
            blocks: NORMAL_INTEREST,
        }
    }
}

#[derive(Error, Debug)]
#[error("expected interest level to be one of: 'low', 'normal', 'high', found {0}")]
pub struct ParseError(String);

impl TryFrom<TopicsOfInterest> for RingsConfig {
    type Error = ParseError;

    fn try_from(topics: TopicsOfInterest) -> Result<Self, Self::Error> {
        fn parse_interest(interest: String) -> Result<InterestLevel, ParseError> {
            match interest.as_str() {
                "low" => Ok(LOW_INTERESET),
                "normal" => Ok(NORMAL_INTEREST),
                "high" => Ok(HIGH_INTEREST),
                _ => Err(ParseError(interest)),
            }
        }

        Ok(Self {
            messages: parse_interest(topics.messages)?,
            blocks: parse_interest(topics.blocks)?,
        })
    }
}

/// This layer is very similar to poldercast::Rings,
/// but allows to set a fixed interest for topics, instead
/// of calculating it based current topology
pub struct Rings {
    /// the max number of entries to add in the list of the view
    rings: poldercast::layer::Rings,
    interest_levels: RingsConfig,
}

impl Rings {
    pub fn new(interest_levels: RingsConfig, rings: poldercast::layer::Rings) -> Self {
        Self {
            interest_levels,
            rings,
        }
    }
}

impl Layer for Rings {
    fn name(&self) -> &'static str {
        "custom::rings"
    }

    fn view(&mut self, builder: &mut ViewBuilder) {
        self.rings.view(builder)
    }

    fn remove(&mut self, id: &keynesis::key::ed25519::PublicKey) {
        self.rings.remove(id)
    }

    fn reset(&mut self) {
        self.rings.reset()
    }

    fn subscribe(&mut self, topic: Topic) {
        self.rings.subscribe(topic)
    }

    fn unsubscribe(&mut self, topic: &Topic) {
        self.rings.unsubscribe(topic)
    }

    fn subscriptions(&self, output: &mut PriorityMap<InterestLevel, Topic>) {
        output.put(self.interest_levels.blocks, topic::BLOCKS);
        output.put(self.interest_levels.messages, topic::MESSAGES);
    }

    fn populate(&mut self, our_profile: &Profile, new_profile: &Profile) {
        self.rings.populate(our_profile, new_profile)
    }
}
