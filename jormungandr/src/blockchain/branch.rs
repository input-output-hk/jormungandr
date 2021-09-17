use crate::blockchain::Ref;
use futures::stream::{FuturesUnordered, StreamExt};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct Branches {
    inner: Arc<RwLock<BranchesData>>,
}

struct BranchesData {
    branches: Vec<Branch>,
}

#[derive(Clone)]
pub struct Branch {
    inner: Arc<RwLock<BranchData>>,
}

/// the data that is contained in a branch
struct BranchData {
    /// reference to the block where the branch points to
    reference: Arc<Ref>,
}

impl Default for Branches {
    fn default() -> Self {
        Self::new()
    }
}

impl Branches {
    pub fn new() -> Self {
        Branches {
            inner: Arc::new(RwLock::new(BranchesData {
                branches: Vec::new(),
            })),
        }
    }

    pub async fn add(&mut self, branch: Branch) {
        let mut guard = self.inner.write().await;
        guard.add(branch);
    }

    pub async fn apply_or_create(&mut self, candidate: Arc<Ref>) -> Branch {
        let maybe_branch = self.apply(Arc::clone(&candidate)).await;
        match maybe_branch {
            Some(branch) => branch,
            None => {
                let maybe_exists = self
                    .branches()
                    .await
                    .into_iter()
                    .find(|branch| branch.hash() == candidate.hash());

                if let Some(branch) = maybe_exists {
                    return Branch::new(branch);
                }
                self.create(candidate).await
            }
        }
    }

    pub async fn branches(&self) -> Vec<Arc<Ref>> {
        let guard = self.inner.read().await;
        guard.branches().await
    }

    async fn apply(&mut self, candidate: Arc<Ref>) -> Option<Branch> {
        let mut guard = self.inner.write().await;
        guard.apply(candidate).await
    }

    async fn create(&mut self, candidate: Arc<Ref>) -> Branch {
        let branch = Branch::new(candidate);
        self.add(branch.clone()).await;
        branch
    }
}

impl BranchesData {
    fn add(&mut self, branch: Branch) {
        self.branches.push(branch)
    }

    async fn apply(&mut self, candidate: Arc<Ref>) -> Option<Branch> {
        let (value, _) = self
            .branches
            .iter_mut()
            .map(|branch| branch.continue_with(Arc::clone(&candidate)))
            .collect::<FuturesUnordered<_>>()
            .filter_map(|updated| Box::pin(async move { updated }))
            .into_future()
            .await;
        value
    }

    async fn branches(&self) -> Vec<Arc<Ref>> {
        self.branches
            .iter()
            .map(|b| b.get_ref())
            // this is done so that inner futures are only polled when they generate wake-up notifications
            .collect::<FuturesUnordered<_>>()
            .collect()
            .await
    }
}

impl Branch {
    pub fn new(reference: Arc<Ref>) -> Self {
        Branch {
            inner: Arc::new(RwLock::new(BranchData::new(reference))),
        }
    }

    pub async fn get_ref(&self) -> Arc<Ref> {
        let guard = self.inner.read().await;
        guard.reference()
    }

    pub async fn update_ref(&mut self, new_ref: Arc<Ref>) -> Arc<Ref> {
        let mut guard = self.inner.write().await;
        guard.update(new_ref)
    }

    async fn continue_with(&mut self, candidate: Arc<Ref>) -> Option<Self> {
        let mut guard = self.inner.write().await;
        if guard.continue_with(candidate) {
            Some(self.clone())
        } else {
            None
        }
    }
}

impl BranchData {
    /// create the branch data with the current `last_updated` to
    /// the current time this function was called
    fn new(reference: Arc<Ref>) -> Self {
        BranchData { reference }
    }

    fn update(&mut self, reference: Arc<Ref>) -> Arc<Ref> {
        std::mem::replace(&mut self.reference, reference)
    }

    fn reference(&self) -> Arc<Ref> {
        Arc::clone(&self.reference)
    }

    fn continue_with(&mut self, candidate: Arc<Ref>) -> bool {
        if self.reference.hash() == candidate.block_parent_hash() {
            let _parent = self.update(candidate);
            true
        } else {
            false
        }
    }
}
