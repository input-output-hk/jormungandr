use crate::blockchain::protocols::Ref;
use tokio::{prelude::*, sync::lock::Lock};

#[derive(Clone)]
pub struct Branches {
    branches: Vec<Branch>,
}

#[derive(Clone)]
pub struct Branch {
    inner: Lock<BranchData>,
}

/// the data that is contained in a branch
struct BranchData {
    /// reference to the block where the branch points to
    reference: Ref,

    last_updated: std::time::SystemTime,
}

impl Branches {
    pub fn new() -> Self {
        Branches {
            branches: Vec::new(),
        }
    }

    pub fn add(&mut self, branch: Branch) {
        self.branches.push(branch)
    }
}

impl Branch {
    pub fn new(reference: Ref) -> Self {
        Branch {
            inner: Lock::new(BranchData::new(reference)),
        }
    }

    pub fn get_ref(&self) -> impl Future<Item = Ref, Error = std::convert::Infallible> {
        let mut branch = self.clone();
        future::poll_fn(move || Ok(branch.inner.poll_lock())).map(|guard| guard.reference().clone())
    }
}

impl BranchData {
    /// create the branch data with the current `last_updated` to
    /// the current time this function was called
    fn new(reference: Ref) -> Self {
        BranchData {
            reference,
            last_updated: std::time::SystemTime::now(),
        }
    }

    fn update(&mut self, reference: Ref) -> Ref {
        let old_reference = std::mem::replace(&mut self.reference, reference);
        self.last_updated = std::time::SystemTime::now();

        old_reference
    }

    fn reference(&self) -> &Ref {
        &self.reference
    }
}
