use crate::blockchain::{Branch, Ref};
use std::{convert::Infallible, sync::Arc};
use tokio::prelude::*;

#[derive(Clone)]
pub struct Tip {
    branch: Branch,
}

impl Tip {
    pub fn new(branch: Branch) -> Self {
        Tip { branch }
    }

    pub fn get_ref(&self) -> impl Future<Item = Arc<Ref>, Error = Infallible> {
        self.branch.get_ref()
    }

    pub fn update_ref(
        &mut self,
        new_ref: Arc<Ref>,
    ) -> impl Future<Item = Arc<Ref>, Error = Infallible> {
        self.branch.update_ref(new_ref)
    }

    pub fn swap(&mut self, mut branch: Branch) -> impl Future<Item = (), Error = Infallible> {
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
