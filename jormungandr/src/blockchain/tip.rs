use crate::blockchain::{Branch, Ref};
use std::sync::Arc;

#[derive(Clone)]
pub struct Tip {
    branch: Branch,
}

impl Tip {
    pub fn new(branch: Branch) -> Self {
        Tip { branch }
    }

    pub async fn get_ref(&self) -> Arc<Ref> {
        self.branch.get_ref().await
    }

    pub async fn update_ref(&mut self, new_ref: Arc<Ref>) -> Arc<Ref> {
        self.branch.update_ref(new_ref).await
    }

    pub async fn swap(&mut self, mut branch: Branch) {
        let mut tip_branch = self.branch.clone();
        let tr = self.branch.get_ref().await;
        let br = branch.update_ref(tr).await;
        tip_branch.update_ref(br).await;
    }

    pub fn branch(&self) -> &Branch {
        &self.branch
    }
}
