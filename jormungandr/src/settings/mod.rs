mod command_arguments;
pub mod start;

pub use self::command_arguments::CommandLine;
pub use self::start::Error;
use crate::blockcfg::HeaderHash;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub enum Block0Info {
    Path(PathBuf, Option<HeaderHash>),
    Hash(HeaderHash),
}
