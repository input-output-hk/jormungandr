use std::{cmp::Ordering, fmt};

#[derive(Clone, Debug)]
pub struct ResourcesUsage {
    cpu_usage: u32,
    memory_usage: u32,
    virtual_memory_usage: u32,
}

impl Eq for ResourcesUsage {}

impl PartialEq for ResourcesUsage {
    fn eq(&self, other: &Self) -> bool {
        self.cpu_usage == other.cpu_usage
            && self.memory_usage == other.memory_usage
            && self.virtual_memory_usage == other.virtual_memory_usage
    }
}

impl PartialOrd for ResourcesUsage {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.usage_indicator().partial_cmp(&other.usage_indicator())
    }
}

impl Ord for ResourcesUsage {
    fn cmp(&self, other: &Self) -> Ordering {
        self.usage_indicator().cmp(&other.usage_indicator())
    }
}

impl fmt::Display for ResourcesUsage {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Usage(CPU: {:.1} %. Memory: {:.1}. Virtual memory: {:.1})",
            self.cpu_usage, self.memory_usage, self.virtual_memory_usage
        )
    }
}

impl ResourcesUsage {
    pub fn new(cpu_usage: u32, memory_usage: u32, virtual_memory_usage: u32) -> Self {
        Self {
            cpu_usage,
            memory_usage,
            virtual_memory_usage,
        }
    }

    pub fn usage_indicator(&self) -> u32 {
        self.cpu_usage() * 10 + self.memory_usage() + self.virtual_memory_usage()
    }

    pub fn cpu_usage(&self) -> u32 {
        self.cpu_usage
    }

    pub fn memory_usage(&self) -> u32 {
        self.memory_usage
    }

    pub fn virtual_memory_usage(&self) -> u32 {
        self.virtual_memory_usage
    }
}
