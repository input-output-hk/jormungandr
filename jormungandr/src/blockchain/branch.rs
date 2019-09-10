use crate::blockchain::Ref;
use std::{convert::Infallible, sync::Arc};
use tokio::{prelude::*, sync::lock::Lock};

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
    reference: Arc<Ref>,

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

    pub fn apply_or_create(
        &mut self,
        candidate: Arc<Ref>,
    ) -> impl Future<Item = Branch, Error = Infallible> {
        let mut branches = self.clone();
        self.apply(Arc::clone(&candidate))
            .and_then(move |opt_branch| {
                if let Some(branch) = opt_branch {
                    future::Either::A(future::ok(branch))
                } else {
                    future::Either::B(branches.create(candidate))
                }
            })
    }

    fn apply(
        &mut self,
        candidate: Arc<Ref>,
    ) -> impl Future<Item = Option<Branch>, Error = Infallible> {
        let mut branches = self.clone();
        future::poll_fn(move || Ok(branches.inner.poll_lock()))
            .and_then(move |mut guard| guard.apply(candidate))
    }

    fn create(&mut self, candidate: Arc<Ref>) -> impl Future<Item = Branch, Error = Infallible> {
        let branch = Branch::new(candidate);
        self.add(branch.clone()).map(move |()| branch)
    }
}

impl BranchesData {
    fn add(&mut self, branch: Branch) {
        self.branches.push(branch)
    }

    pub fn apply(
        &mut self,
        candidate: Arc<Ref>,
    ) -> impl Future<Item = Option<Branch>, Error = Infallible> {
        stream::futures_unordered(
            self.branches
                .iter_mut()
                .map(|branch| branch.continue_with(Arc::clone(&candidate))),
        )
        .filter_map(|updated| updated)
        .into_future()
        .map_err(|(e, _)| e)
        .map(|(v, _)| v)
    }
}

impl Branch {
    pub fn new(reference: Arc<Ref>) -> Self {
        Branch {
            inner: Lock::new(BranchData::new(reference)),
        }
    }

    pub fn get_ref(&self) -> impl Future<Item = Arc<Ref>, Error = Infallible> {
        let mut branch = self.inner.clone();
        future::poll_fn(move || Ok(branch.poll_lock())).map(|guard| guard.reference().clone())
    }

    pub fn update_ref(
        &mut self,
        new_ref: Arc<Ref>,
    ) -> impl Future<Item = Arc<Ref>, Error = Infallible> {
        let mut branch = self.inner.clone();
        future::poll_fn(move || Ok(branch.poll_lock())).map(move |mut guard| guard.update(new_ref))
    }

    fn continue_with(
        &mut self,
        candidate: Arc<Ref>,
    ) -> impl Future<Item = Option<Self>, Error = Infallible> {
        let clone_branch = self.clone();
        let mut branch = self.inner.clone();
        future::poll_fn(move || Ok(branch.poll_lock()))
            .map(move |mut guard| guard.continue_with(candidate))
            .map(move |r| if r { Some(clone_branch) } else { None })
    }
}

impl BranchData {
    /// create the branch data with the current `last_updated` to
    /// the current time this function was called
    fn new(reference: Arc<Ref>) -> Self {
        BranchData {
            reference,
            last_updated: std::time::SystemTime::now(),
        }
    }

    fn update(&mut self, reference: Arc<Ref>) -> Arc<Ref> {
        let old_reference = std::mem::replace(&mut self.reference, reference);
        self.last_updated = std::time::SystemTime::now();

        old_reference
    }

    fn reference(&self) -> Arc<Ref> {
        Arc::clone(&self.reference)
    }

    fn continue_with(&mut self, candidate: Arc<Ref>) -> bool {
        if &self.reference.hash() == candidate.block_parent_hash() {
            let _parent = self.update(candidate);
            true
        } else {
            false
        }
    }
}
