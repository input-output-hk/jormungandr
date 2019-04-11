use std::process::Child;

/// Struct ensures child process is killed if leaves given scope
///
pub struct ProcessKillGuard {
    child: Child,
}

impl ProcessKillGuard {
    pub fn new(child: Child) -> ProcessKillGuard {
        ProcessKillGuard { child }
    }
}

impl Drop for ProcessKillGuard {
    fn drop(&mut self) {
        match self.child.kill() {
            Err(e) => println!("Could not kill child process: {}", e),
            Ok(_) => println!("Successfully killed child process"),
        }
    }
}
