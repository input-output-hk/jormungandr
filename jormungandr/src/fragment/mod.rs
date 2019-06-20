mod entry;
mod logs;
mod pool;
mod process;
pub mod selection;

pub use self::entry::PoolEntry;
pub use self::logs::Logs;
pub use self::pool::Pool;
pub use self::process::Process;

use crate::blockcfg::{Message, MessageId};

pub type FragmentId = MessageId;
pub type Fragment = Message;
