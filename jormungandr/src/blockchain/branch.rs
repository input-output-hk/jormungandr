use crate::blockchain::Ref;
use std::sync::Arc;

/// the data that is contained in a branch
#[derive(Clone)]
pub struct Branch {
    /// reference to the block where the branch points to
    reference: Arc<Ref>,
}

impl Branch {
    /// create the branch data with the current `last_updated` to
    /// the current time this function was called
    pub fn new(reference: Arc<Ref>) -> Self {
        Branch { reference }
    }

    pub fn get_ref(&self) -> Arc<Ref> {
        Arc::clone(&self.reference)
    }

    pub fn into_ref(self) -> Arc<Ref> {
        self.reference
    }
}
