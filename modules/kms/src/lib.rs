mod id;
mod manager;
mod query;

use std::collections::HashMap;
use thiserror::Error;

pub mod managers;
pub use self::{
    id::Id,
    manager::{KeyManager, ManagerError},
    query::{Query, Schedule},
};

#[derive(Debug, Error)]
pub enum Error {
    #[error("Cannot register the key manager")]
    CannotRegister {
        #[source]
        reason: ManagerError,
    },
    #[error("Cannot un-register the key manager {id}")]
    CannotUnRegister {
        id: Id,
        #[source]
        reason: ManagerError,
    },
    #[error("Cannot query the key manager {id}")]
    CannotQuery {
        id: Id,
        #[source]
        reason: ManagerError,
    },
}

#[derive(Default)]
pub struct KMS {
    keys: HashMap<Id, KeyManager>,
}

impl KMS {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn contains(&self, id: &Id) -> bool {
        self.keys.contains_key(id)
    }

    pub fn register(&mut self, id: Id, mut manager: KeyManager) -> Result<Id, Error> {
        manager
            .register()
            .map_err(|reason| Error::CannotRegister { reason })?;
        self.keys.insert(id, manager);
        Ok(id)
    }

    pub fn un_register(&mut self, id: Id) -> Result<(), Error> {
        if let Some(mut manager) = self.keys.remove(&id) {
            manager
                .un_register()
                .map_err(|reason| Error::CannotUnRegister { id, reason })?;
        }
        Ok(())
    }

    /// list all the ID present in the KMS
    // TODO: wrap the Keys iterator to an IdIter type. so we don't expose internals
    pub fn ids(&self) -> std::collections::hash_map::Keys<'_, Id, KeyManager> {
        self.keys.keys()
    }

    pub fn query(&mut self, id: Id, query: Query) -> Result<(), Error> {
        if let Some(manager) = self.keys.get_mut(&id) {
            manager
                .query(query)
                .map_err(|reason| Error::CannotQuery { id, reason })?;
        }

        Ok(())
    }
}
