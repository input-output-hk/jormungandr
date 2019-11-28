use crate::start_up::Error;
use std::fmt::{self, Display, Formatter};

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub open_files_limit: u64,
    pub cpu_usage_limit: u64,
}

impl Diagnostic {
    #[cfg(unix)]
    pub fn new() -> Result<Self, Error> {
        Ok(Self {
            open_files_limit: getrlimit(libc::RLIMIT_NOFILE)
                .map_err(|e| Error::DiagnosticError { source: e })?,
            cpu_usage_limit: getrlimit(libc::RLIMIT_CPU)
                .map_err(|e| Error::DiagnosticError { source: e })?,
        })
    }
}

impl Display for Diagnostic {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        write!(
            formatter,
            "limit for open files (RLIMIT_NOFILE): {}",
            self.open_files_limit
        )?;
        write!(
            formatter,
            "; limit for CPU usage (RLIMIT_CPU): {}",
            self.cpu_usage_limit
        )
    }
}

#[cfg(target_os = "macos")]
type RlimitResource = i32;

#[cfg(all(unix, not(target_os = "macos")))]
type RlimitResource = u32;

#[cfg(unix)]
fn getrlimit(resource: RlimitResource) -> Result<u64, nix::Error> {
    use libc::rlimit;

    let mut limits = rlimit {
        rlim_cur: 0,
        rlim_max: 0,
    };

    let retcode = unsafe { libc::getrlimit(resource, &mut limits as *mut rlimit) };
    nix::errno::Errno::result(retcode)?;

    Ok(limits.rlim_cur)
}
