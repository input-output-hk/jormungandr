use crate::{managers, Query};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ManagerError {}

pub enum KeyManager {
    InMemory(managers::InMemory),
}

impl KeyManager {
    pub(crate) fn register(&mut self) -> Result<(), ManagerError> {
        match self {
            Self::InMemory(_in_memory) => Ok(()),
        }
    }

    pub(crate) fn un_register(&mut self) -> Result<(), ManagerError> {
        match self {
            Self::InMemory(_in_memory) => Ok(()),
        }
    }

    pub(crate) fn query(&mut self, query: Query) -> Result<(), ManagerError> {
        match self {
            Self::InMemory(in_memory) => {
                in_memory.query(query);
                Ok(())
            }
        }
    }
}
