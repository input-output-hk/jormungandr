use crate::blockchain::protocols::Ref;
use tokio::{
    prelude::*,
    sync::lock::{Lock, LockGuard},
};

#[derive(Clone)]
pub struct Branches {
    branches: Vec<Branch>,
}

#[derive(Clone)]
pub struct Branch {
    inner: Lock<BranchData>,
}

/// the data that is contained in a branch
pub struct BranchData {
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

    /// try to acquire a lock on the given Branch in an async way.
    ///
    #[inline]
    pub fn poll_lock<E>(&mut self) -> Poll<LockGuard<BranchData>, E> {
        Ok(self.inner.poll_lock())
    }

    pub fn get_ref(&self) -> impl Future<Item = Ref, Error = std::convert::Infallible> {
        let mut branch = self.clone();
        future::poll_fn(move || branch.poll_lock()).map(|guard| guard.reference().clone())
    }
}

impl BranchData {
    /// create the branch data with the current `last_updated` to
    /// the current time this function was called
    #[inline(always)]
    fn new(reference: Ref) -> Self {
        BranchData {
            reference,
            last_updated: std::time::SystemTime::now(),
        }
    }

    #[inline(always)]
    pub fn update(&mut self, reference: Ref) -> Ref {
        let old_reference = std::mem::replace(&mut self.reference, reference);
        self.last_updated = std::time::SystemTime::now();

        old_reference
    }

    #[inline(always)]
    pub fn reference(&self) -> &Ref {
        &self.reference
    }
}
