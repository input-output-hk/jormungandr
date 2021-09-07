use chain_core::property::FromStr;
use chain_impl_mockchain::block::{BlockDate, BlockDateParseError};

pub fn parse_shift(from: &str) -> Result<(BlockDate, bool), BlockDateParseError> {
    if let Some(stripped) = from.strip_prefix('~') {
        BlockDate::from_str(stripped).map(|d| (d, true))
    } else {
        BlockDate::from_str(from).map(|d| (d, false))
    }
}
