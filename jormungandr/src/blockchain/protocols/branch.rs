use crate::blockchain::protocols::Ref;
use tokio::{prelude::*, sync::lock::Lock};

use std::convert::Infallible;

#[derive(Clone)]
pub struct Branches {
    inner: Lock<BranchesData>,
}

struct BranchesData {
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
            inner: Lock::new(BranchesData {
                branches: Vec::new(),
            }),
        }
    }

    pub fn add(&mut self, branch: Branch) -> impl Future<Item = (), Error = Infallible> {
        let mut branches = self.clone();
        future::poll_fn(move || Ok(branches.inner.poll_lock()))
            .map(move |mut guard| guard.add(branch))
    }
}

impl BranchesData {
    fn add(&mut self, branch: Branch) {
        self.branches.push(branch)
    }
}

impl Branch {
    pub fn new(reference: Ref) -> Self {
        Branch {
            inner: Lock::new(BranchData::new(reference)),
        }
    }

    pub fn get_ref(&self) -> impl Future<Item = Ref, Error = Infallible> {
        let mut branch = self.inner.clone();
        future::poll_fn(move || Ok(branch.poll_lock())).map(|guard| guard.reference().clone())
    }

    pub fn update_ref(&mut self, new_ref: Ref) -> impl Future<Item = Ref, Error = Infallible> {
        let mut branch = self.inner.clone();
        future::poll_fn(move || Ok(branch.poll_lock())).map(move |mut guard| guard.update(new_ref))
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
