use crate::testing::measurement::{marker::ResourcesUsage, status::Status, thresholds::Thresholds};
use std::{cmp::Ordering, fmt};

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct NamedProcess {
    name: String,
    id: usize,
}

impl NamedProcess {
    pub fn new(name: String, id: usize) -> Self {
        Self { name, id }
    }

    pub fn name(&self) -> String {
        self.name.to_string()
    }

    pub fn id(&self) -> usize {
        self.id
    }
}

impl fmt::Display for NamedProcess {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "id: {}, alias: {}", self.id, self.name)
    }
}

#[derive(Clone, Debug)]
pub struct Consumption {
    average_usage: ResourcesUsage,
}

impl fmt::Display for Consumption {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.average_usage)
    }
}

impl PartialEq for Consumption {
    fn eq(&self, other: &Self) -> bool {
        self.average_usage == other.average_usage
    }
}

impl PartialOrd for Consumption {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.average_usage.partial_cmp(&other.average_usage)
    }
}

impl Consumption {
    pub fn new(markers: Vec<ResourcesUsage>) -> Self {
        Self {
            average_usage: Self::average_resource_usage(markers),
        }
    }

    fn average_resource_usage(markers: Vec<ResourcesUsage>) -> ResourcesUsage {
        let average_cpu = Self::median(markers.iter().map(|x| x.cpu_usage()).collect());
        let average_memory = Self::median(markers.iter().map(|x| x.memory_usage()).collect());
        let average_virtual_memory =
            Self::median(markers.iter().map(|x| x.virtual_memory_usage()).collect());
        ResourcesUsage::new(average_cpu, average_memory, average_virtual_memory)
    }

    fn median(mut markers: Vec<u32>) -> u32 {
        markers.sort();
        let mid = markers.len() / 2;
        *markers.get(mid).unwrap()
    }

    pub fn against(&self, thresholds: &Thresholds<Self>) -> Status {
        let green = thresholds.green_threshold();
        let yellow = thresholds.yellow_threshold();

        if *self <= green {
            return Status::Green;
        }
        if *self <= yellow {
            return Status::Yellow;
        }
        Status::Red
    }
}
