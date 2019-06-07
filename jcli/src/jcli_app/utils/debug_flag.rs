use structopt::StructOpt;

#[derive(StructOpt)]
pub struct DebugFlag {
    /// print additional debug information to stderr.
    /// The output format is intentionally undocumented and unstable
    #[structopt(long)]
    debug: bool,
}

impl DebugFlag {
    pub fn debug_writer(&self) -> Option<impl std::io::Write> {
        match self.debug {
            true => Some(std::io::stderr()),
            false => None,
        }
    }
}
