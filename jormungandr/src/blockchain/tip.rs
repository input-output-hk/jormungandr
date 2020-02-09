use crate::blockchain::{Branch, Ref};
use std::{convert::Infallible, sync::Arc};
use tokio::prelude::Future as Future01;
use tokio_compat::prelude::*;

#[derive(Clone)]
pub struct Tip {
    branch: Branch,
}

impl Tip {
    pub fn new(branch: Branch) -> Self {
        Tip { branch }
    }

    pub async fn get_ref_std(&self) -> Arc<Ref> {
        let r: Result<_, ()> = self.branch.get_ref().compat().await;
        r.unwrap()
    }

    pub async fn update_ref_std(&mut self, new_ref: Arc<Ref>) -> Arc<Ref> {
        let r = self.branch.update_ref(new_ref).compat().await.unwrap();
        r
    }

    pub async fn swap_std(&mut self, mut branch: Branch) {
        let mut tip_branch = self.branch.clone();
        let tr = self.branch().get_ref_std().await;
        let br = branch.update_ref_std(tr).await;
        tip_branch.update_ref_std(br);
    }

    pub fn get_ref<E>(&self) -> impl Future01<Item = Arc<Ref>, Error = E> {
        self.branch.get_ref()
    }

    pub fn update_ref(
        &mut self,
        new_ref: Arc<Ref>,
    ) -> impl Future01<Item = Arc<Ref>, Error = Infallible> {
        self.branch.update_ref(new_ref)
    }

    pub fn swap(&mut self, mut branch: Branch) -> impl Future01<Item = (), Error = Infallible> {
        let mut tip_branch = self.branch.clone();
        self.branch()
            .get_ref()
            .and_then(move |tr| branch.update_ref(tr))
            .and_then(move |br| tip_branch.update_ref(br))
            .map(|_| ())
    }

    pub fn branch(&self) -> &Branch {
        &self.branch
    }
}
