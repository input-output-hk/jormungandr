use bytesize::ByteSize;
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
        let cpu_cmp = self.cpu_usage().partial_cmp(&other.cpu_usage()).unwrap();
        let memory_cmp = self
            .memory_usage()
            .partial_cmp(&other.memory_usage())
            .unwrap();
        let virtual_memory_cmp = self
            .virtual_memory_usage()
            .partial_cmp(&other.virtual_memory_usage())
            .unwrap();

        Some(cpu_cmp.then(memory_cmp).then(virtual_memory_cmp))
    }
}

impl Ord for ResourcesUsage {
    fn cmp(&self, other: &Self) -> Ordering {
        let cpu_cmp = self.cpu_usage().cmp(&other.cpu_usage());
        let memory_cmp = self.memory_usage().cmp(&other.memory_usage());
        let virtual_memory_cmp = self
            .virtual_memory_usage()
            .cmp(&other.virtual_memory_usage());

        cpu_cmp.then(memory_cmp).then(virtual_memory_cmp)
    }
}

impl fmt::Display for ResourcesUsage {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let memory_usage = ByteSize::kib(self.memory_usage.into()).to_string();
        let virtual_memory_usage = ByteSize::kib(self.virtual_memory_usage.into()).to_string();

        write!(
            f,
            "(CPU: {:.1} %. Mem: {}. V_Mem: {})",
            self.cpu_usage, memory_usage, virtual_memory_usage
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
