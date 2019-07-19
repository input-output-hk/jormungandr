mod command_arguments;
pub mod logging;
pub mod start;

pub use self::command_arguments::CommandLine;
pub use self::start::Error;
use crate::blockcfg::HeaderHash;
use std::path::PathBuf;

lazy_static! {
    pub static ref LOG_FILTER_LEVEL_POSSIBLE_VALUES: Vec<&'static str> = {
        slog::LOG_LEVEL_NAMES
            .iter()
            .map(|name| name.to_ascii_lowercase())
            .map(|name| &*Box::leak(name.into_boxed_str()))
            .collect()
    };
}

#[derive(Clone, Debug)]
pub enum Block0Info {
    Path(PathBuf),
    Hash(HeaderHash),
}
