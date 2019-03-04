use super::content::{LeafIterator, KV};
use super::hash::{Hash, HashedKey, Hasher};
use super::node::{
    insert_rec, lookup_one, remove_eq_rec, remove_rec, size_rec, update_rec, Entry, LookupRet,
    Node, NodeIter,
};
pub use super::operation::{InsertError, RemoveError, UpdateError};
use super::sharedref::SharedRef;
use std::iter::FromIterator;
use std::marker::PhantomData;
use std::mem::swap;

#[derive(Clone)]
pub struct Hamt<H: Hasher + Default, K: PartialEq + Eq + Hash, V> {
    root: Node<K, V>,
    hasher: PhantomData<H>,
}

pub struct HamtIter<'a, K, V> {
    stack: Vec<NodeIter<'a, K, V>>,
    content: Option<LeafIterator<'a, K, V>>,
}

impl<H: Hasher + Default, K: Eq + Hash, V> Hamt<H, K, V> {
    pub fn new() -> Self {
        Hamt {
            root: Node::new(),
            hasher: PhantomData,
        }
    }

    pub fn size(&self) -> usize {
        size_rec(&self.root)
    }
}

impl<H: Hasher + Default, K: Eq + Hash, V> Hamt<H, K, V> {
    pub fn insert(&self, k: K, v: V) -> Result<Self, InsertError> {
        let h = HashedKey::compute(self.hasher, &k);
        let kv = SharedRef::new(KV::new(k, v));
        let newroot = insert_rec(&self.root, &h, 0, kv)?;
        Ok(Hamt {
            root: newroot,
            hasher: PhantomData,
        })
    }
}

impl<H: Hasher + Default, K: Eq + Hash, V: PartialEq> Hamt<H, K, V> {
    pub fn remove_match(&self, k: &K, v: &V) -> Result<Self, RemoveError> {
        let h = HashedKey::compute(self.hasher, &k);
        let newroot = remove_eq_rec(&self.root, &h, 0, k, v)?;
        match newroot {
            None => Ok(Self::new()),
            Some(r) => Ok(Hamt {
                root: r,
                hasher: PhantomData,
            }),
        }
    }
}

impl<H: Hasher + Default, K: Eq + Hash, V> Hamt<H, K, V> {
    pub fn remove(&self, k: &K) -> Result<Self, RemoveError> {
        let h = HashedKey::compute(self.hasher, &k);
        let newroot = remove_rec(&self.root, &h, 0, k)?;
        match newroot {
            None => Ok(Self::new()),
            Some(r) => Ok(Hamt {
                root: r,
                hasher: PhantomData,
            }),
        }
    }
}

impl<H: Hasher + Default, K: Eq + Hash + Clone, V> Hamt<H, K, V> {
    pub fn update<F, U>(&self, k: &K, f: F) -> Result<Self, UpdateError<U>>
    where
        F: FnOnce(&V) -> Result<Option<V>, U>,
    {
        let h = HashedKey::compute(self.hasher, &k);
        let newroot = update_rec(&self.root, &h, 0, k, f)?;
        match newroot {
            None => Ok(Self::new()),
            Some(r) => Ok(Hamt {
                root: r,
                hasher: PhantomData,
            }),
        }
    }
}

impl<H: Hasher + Default, K: Hash + Eq, V> Hamt<H, K, V> {
    pub fn lookup(&self, k: &K) -> Option<&V> {
        let h = HashedKey::compute(self.hasher, &k);
        let mut n = &self.root;
        let mut lvl = 0;
        loop {
            match lookup_one(n, &h, lvl, k) {
                LookupRet::NotFound => return None,
                LookupRet::Found(v) => return Some(v),
                LookupRet::ContinueIn(subnode) => {
                    lvl += 1;
                    n = &subnode;
                }
            }
        }
    }
    pub fn iter(&self) -> HamtIter<K, V> {
        HamtIter {
            stack: vec![self.root.iter()],
            content: None,
        }
    }
}

impl<'a, K, V> Iterator for HamtIter<'a, K, V> {
    type Item = (&'a K, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let mut x = None;
            swap(&mut self.content, &mut x);
            match x {
                Some(mut iter) => match iter.next() {
                    None => self.content = None,
                    Some(ref o) => {
                        self.content = Some(iter);
                        return Some((o.get_key(), o.get_value()));
                    }
                },
                None => match self.stack.last_mut() {
                    None => return None,
                    Some(l) => match l.next() {
                        None => {
                            self.stack.pop();
                        }
                        Some(o) => match o.as_ref() {
                            &Entry::SubNode(ref sub) => self.stack.push(sub.iter()),
                            &Entry::Leaf(ref leaf) => self.content = Some(leaf.iter()),
                        },
                    },
                },
            }
        }
    }
}

impl<H: Default + Hasher, K: Eq + Hash, V> FromIterator<(K, V)> for Hamt<H, K, V> {
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        let mut h = Hamt::new();
        for (k, v) in iter {
            match h.insert(k, v) {
                Err(_) => {}
                Ok(newh) => h = newh,
            }
        }
        h
    }
}
