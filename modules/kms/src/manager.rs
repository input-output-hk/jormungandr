use crate::{managers, Query};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ManagerError {}

pub struct KeyManager {
    inner: Inner,
}

enum Inner {
    InMemory(managers::InMemory),
}

impl KeyManager {
    pub fn in_memory(in_memory: managers::InMemory) -> Self {
        Self {
            inner: Inner::InMemory(in_memory),
        }
    }

    pub(crate) fn register(&mut self) -> Result<(), ManagerError> {
        self.inner.register()
    }

    pub(crate) fn un_register(&mut self) -> Result<(), ManagerError> {
        self.inner.un_register()
    }

    pub(crate) fn query(&mut self, query: Query) -> Result<(), ManagerError> {
        self.inner.query(query)
    }
}

impl Inner {
    fn register(&mut self) -> Result<(), ManagerError> {
        match self {
            Self::InMemory(_in_memory) => Ok(()),
        }
    }

    fn un_register(&mut self) -> Result<(), ManagerError> {
        match self {
            Self::InMemory(_in_memory) => Ok(()),
        }
    }

    fn query(&mut self, query: Query) -> Result<(), ManagerError> {
        match self {
            Self::InMemory(in_memory) => {
                in_memory.query(query);
                Ok(())
            }
        }
    }
}
