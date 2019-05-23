mod entry;
mod log;
mod logs;
mod pool;
mod process;

pub use self::entry::PoolEntry;
pub use self::log::{Log, Origin, Status};
pub use self::logs::Logs;
pub use self::pool::Pool;
pub use self::process::Process;

use crate::blockcfg::{Message, MessageId};

pub type FragmentId = MessageId;
pub type Fragment = Message;
