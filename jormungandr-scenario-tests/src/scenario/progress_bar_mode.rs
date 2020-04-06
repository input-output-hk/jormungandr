use crate::test::Result;
use std::fmt;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ProgressBarMode {
    Monitor,
    Standard,
    None,
}

pub fn parse_progress_bar_mode_from_str(progress_bar_mode: &str) -> Result<ProgressBarMode> {
    let progress_bar_mode_lowercase: &str = &progress_bar_mode.to_lowercase();
    match progress_bar_mode_lowercase {
        "standard" => Ok(ProgressBarMode::Standard),
        "none" => Ok(ProgressBarMode::None),
        _ => Ok(ProgressBarMode::Monitor),
    }
}

impl fmt::Display for ProgressBarMode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
