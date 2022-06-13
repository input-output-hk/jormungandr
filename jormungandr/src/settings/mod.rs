mod command_arguments;
pub mod logging;
pub mod start;

pub use self::{command_arguments::CommandLine, start::Error};
use crate::blockcfg::HeaderHash;
use std::path::PathBuf;

lazy_static! {
    pub static ref LOG_FILTER_LEVEL_POSSIBLE_VALUES: Vec<&'static str> = {
        [
            tracing::metadata::LevelFilter::OFF,
            tracing::metadata::LevelFilter::TRACE,
            tracing::metadata::LevelFilter::DEBUG,
            tracing::metadata::LevelFilter::INFO,
            tracing::metadata::LevelFilter::WARN,
            tracing::metadata::LevelFilter::ERROR,
        ]
        .iter()
        .map(|name| name.to_string().to_ascii_lowercase())
        .map(|name| &*Box::leak(name.into_boxed_str()))
        .collect()
    };
}

#[derive(Clone, Debug)]
pub enum Block0Info {
    Path(PathBuf, Option<HeaderHash>),
    Hash(HeaderHash),
}
