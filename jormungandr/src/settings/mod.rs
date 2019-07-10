mod command_arguments;
pub mod logging;
pub mod start;

pub use self::command_arguments::CommandLine;
pub use self::start::Error;
use crate::blockcfg::HeaderHash;
use slog::FilterLevel;
use std::path::PathBuf;

const LOG_FILTER_LEVEL_POSSIBLE_VALUES: &[&'static str] =
    &["off", "critical", "error", "warn", "info", "debug", "trace"];

// TODO remove and switch to FilterLevel::as_str() when it's released
fn filter_level_to_str(filter_level: FilterLevel) -> &'static str {
    LOG_FILTER_LEVEL_POSSIBLE_VALUES[filter_level.as_usize()]
}

#[derive(Clone, Debug)]
pub enum Block0Info {
    Path(PathBuf),
    Hash(HeaderHash),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_level_to_str_tests() {
        assert_filter_level_to_str(FilterLevel::Off);
        assert_filter_level_to_str(FilterLevel::Critical);
        assert_filter_level_to_str(FilterLevel::Error);
        assert_filter_level_to_str(FilterLevel::Warning);
        assert_filter_level_to_str(FilterLevel::Info);
        assert_filter_level_to_str(FilterLevel::Debug);
        assert_filter_level_to_str(FilterLevel::Trace);
    }

    fn assert_filter_level_to_str(level: FilterLevel) {
        println!("Testing for level {:?}", level);
        let string = filter_level_to_str(level);
        let new_level = string.parse().expect("Failed to parse");
        assert_eq!(level, new_level, "Invalid parse value");
        println!("Testing for level {:?} succeeded!", level);
    }
}
