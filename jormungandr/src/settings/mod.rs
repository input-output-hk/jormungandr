mod command_arguments;
pub mod logging;
pub mod start;

pub use self::command_arguments::CommandLine;
pub use self::start::Error;
use crate::blockcfg::HeaderHash;
use std::path::PathBuf;

const LOG_FILTER_LEVEL_POSSIBLE_VALUES: &[&'static str] =
    &["off", "critical", "error", "warn", "info", "debug", "trace"];

#[derive(Clone, Debug)]
pub enum Block0Info {
    Path(PathBuf),
    Hash(HeaderHash),
}
