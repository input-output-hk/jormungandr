use crate::start_up::NodeStorage;

use tokio::prelude::*;
use tokio::sync::lock::Lock;

#[derive(Clone)]
pub struct Storage {
    inner: Lock<NodeStorage>,
}

impl Storage {
    pub fn new(storage: NodeStorage) -> Self {
        Storage {
            inner: Lock::new(storage),
        }
    }
}
