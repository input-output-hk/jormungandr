pub mod process;
pub mod selection;

pub use self::process::leadership_task;
pub use self::selection::{IsLeading, Selection};

use super::secure::NodePublic;
use super::settings::Consensus;
