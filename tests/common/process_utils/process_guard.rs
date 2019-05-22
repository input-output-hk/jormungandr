#![allow(dead_code)]

use std::process::Child;

/// Struct ensures child process is killed if leaves given scope

#[derive(Debug)]
pub struct ProcessKillGuard {
    pub child: Child,
    description: String,
}

impl ProcessKillGuard {
    pub fn new(child: Child, description: String) -> ProcessKillGuard {
        ProcessKillGuard { child, description }
    }
}

impl Drop for ProcessKillGuard {
    fn drop(&mut self) {
        match self.child.kill() {
            Err(e) => println!("Could not kill {}: {}", self.description, e),
            Ok(_) => println!("Successfully killed {}", self.description),
        }
    }
}
