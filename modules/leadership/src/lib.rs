use chain_impl_mockchain::header::BlockDate;
use kms::{Id, Schedule};
use std::collections::{BTreeMap, HashMap};

const EXPLORATORY_LENGTH: usize = 20;

/// the leadership plan for the given leader ID
pub struct LeadershipPlan {
    id: Id,
    query_counter: usize,
    schedules: BTreeMap<BlockDate, Schedule>,
}

#[derive(Default)]
pub struct Leadership {
    plans: HashMap<Id, LeadershipPlan>,
}

impl Leadership {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn ids(&self) -> impl ExactSizeIterator<Item = &Id> {
        self.plans.keys()
    }

    pub fn register(&mut self, id: Id) -> Option<LeadershipPlan> {
        self.plans.insert(id, LeadershipPlan::new(id))
    }

    pub fn un_register(&mut self, id: &Id) -> Option<LeadershipPlan> {
        self.plans.remove(id)
    }
}

impl LeadershipPlan {
    pub fn new(id: Id) -> Self {
        Self {
            id,
            query_counter: 0,
            schedules: BTreeMap::new(),
        }
    }

    pub fn id(&self) -> &Id {
        &self.id
    }

    /// clear all the schedules
    pub fn clear(&mut self) {
        self.schedules.clear()
    }

    /// add a schedule to this leader
    pub fn schedule(&mut self, schedule: Schedule) {
        let date = schedule.date;

        self.schedules.insert(date, schedule);
    }

    /// unschedule a given schedule
    pub fn unschedule(&mut self, schedule: &BlockDate) -> Option<Schedule> {
        self.schedules.remove(schedule)
    }

    /// peek the next schedule (if any)
    pub fn next_schedule(&self) -> Option<&Schedule> {
        self.schedules.values().next()
    }

    /// peek the last schedule (if any at all)
    pub fn last_schedule(&self) -> Option<&Schedule> {
        self.schedules.values().next_back()
    }

    /// pop the next schedule
    pub fn pop_next_schedule(&mut self) -> Option<Schedule> {
        let schedule = self.next_schedule()?.date;

        self.unschedule(&schedule)
    }

    pub fn advice_schedule_exploration_length(&mut self) -> usize {
        self.query_counter += 1;
        self.query_counter * EXPLORATORY_LENGTH
    }
}
