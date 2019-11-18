use super::super::bitmap::{ArrayIndex, SmallBitmap};
use super::super::content::{LeafContent, KV};
use super::super::hash::{HashedKey, LevelIndex};
use super::super::helper;
use super::super::operation::*;
use super::super::sharedref::SharedRef;

use std::slice;

/// Node of the Hash Array Mapped Trie
///
/// The bitmap is indexed by a HashSubgroup
/// and give an entry
#[derive(Clone)]
pub struct Node<K, V> {
    pub bitmap: SmallBitmap,
    pub children: Box<[SharedRef<Entry<K, V>>]>,
}

pub type NodeIter<'a, K, V> = slice::Iter<'a, SharedRef<Entry<K, V>>>;

pub enum Entry<K, V> {
    Leaf(LeafContent<K, V>),
    SubNode(Node<K, V>),
}

impl<K, V> Node<K, V> {
    pub fn new() -> Self {
        Node {
            bitmap: SmallBitmap::new(),
            children: Vec::with_capacity(0).into(),
        }
    }

    pub fn singleton(idx: LevelIndex, child: SharedRef<Entry<K, V>>) -> Self {
        Node {
            bitmap: SmallBitmap::once(idx),
            children: vec![child].into(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.bitmap.is_empty()
    }

    pub fn number_children(&self) -> usize {
        self.bitmap.present()
    }

    pub fn get_child(&self, at: ArrayIndex) -> &SharedRef<Entry<K, V>> {
        assert_eq!(at.is_not_found(), false);
        &self.children[at.get_found()]
    }

    pub fn set_at(&self, idx: LevelIndex, child: SharedRef<Entry<K, V>>) -> Self {
        assert_eq!(self.bitmap.is_set(idx), false);

        let pos = self.bitmap.get_sparse_pos(idx);
        let v = helper::clone_array_and_insert_at_pos(&self.children, child, pos.get_found());

        Node {
            bitmap: self.bitmap.set_index(idx),
            children: v,
        }
    }

    pub fn clear_at(&self, idx: LevelIndex) -> Option<Self> {
        assert_eq!(self.bitmap.is_set(idx), true);
        let new_bitmap = self.bitmap.clear_index(idx);
        if new_bitmap.is_empty() {
            None
        } else {
            // use the old bitmap to locate the element
            let pos = self.bitmap.get_sparse_pos(idx);
            let v = helper::clone_array_and_remove_at_pos(&self.children, pos.get_found());

            Some(Node {
                bitmap: new_bitmap,
                children: v,
            })
        }
    }

    pub fn replace_at(&self, idx: ArrayIndex, child: SharedRef<Entry<K, V>>) -> Self {
        // with the raw index should have:
        // assert_eq!(self.bitmap.is_set(idx), true);

        let mut v = self.children.clone();
        v[idx.get_found()] = child;

        Node {
            bitmap: self.bitmap.clone(),
            children: v,
        }
    }

    pub fn clear_or_replace_at(
        &self,
        idx: LevelIndex,
        child: Option<SharedRef<Entry<K, V>>>,
    ) -> Option<Self> {
        assert_eq!(self.bitmap.is_set(idx), true);
        match child {
            None => self.clear_at(idx),
            Some(v) => {
                let aidx = self.bitmap.get_index_sparse(idx);
                Some(self.replace_at(aidx, v))
            }
        }
    }

    pub fn iter(&self) -> NodeIter<K, V> {
        self.children.iter()
    }
}

// Insert leaf recursively, settings parents node back to cope with the change
//
// this is guaranteed by the trie design not to recurse forever, because at some
// point the hashedkey value being shifted by level_index will match to 0,
// creating Leaf and Collision node instead of Subnode.
pub fn insert_rec<K: PartialEq, V>(
    node: &Node<K, V>,
    h: HashedKey,
    lvl: usize,
    k: K,
    v: V,
) -> Result<Node<K, V>, InsertError> {
    let level_hash = h.level_index(lvl);
    let idx = node.bitmap.get_index_sparse(level_hash);
    if idx.is_not_found() {
        let kv = SharedRef::new(KV::new(k, v));
        let content = LeafContent::single(h, kv);
        let e = SharedRef::new(Entry::Leaf(content));
        Ok(node.set_at(level_hash, e))
    } else {
        match &(node.get_child(idx)).as_ref() {
            &Entry::Leaf(ref content) => {
                // in case of same hash, then we append to the collision type
                // otherwise we create a new subnode
                if content.hashed == h {
                    let kv = SharedRef::new(KV::new(k, v));
                    let newent = Entry::Leaf(content.add(kv)?);
                    let e = SharedRef::new(newent);
                    Ok(node.replace_at(idx, e))
                } else {
                    let leaf_idx = content.hashed.level_index(lvl + 1);
                    let entry_next_idx = h.level_index(lvl + 1);
                    let subnode = Node::singleton(leaf_idx, SharedRef::clone(node.get_child(idx)));

                    if entry_next_idx != leaf_idx {
                        let kv = SharedRef::new(KV::new(k, v));
                        let subnode = subnode.set_at(
                            entry_next_idx,
                            SharedRef::new(Entry::Leaf(LeafContent::single(h, kv))),
                        );
                        Ok(node.replace_at(idx, SharedRef::new(Entry::SubNode(subnode))))
                    } else {
                        let r = insert_rec(&subnode, h, lvl + 1, k, v)?;
                        let e = SharedRef::new(Entry::SubNode(r));
                        Ok(node.replace_at(idx, e))
                    }
                }
            }
            &Entry::SubNode(sub) => {
                if lvl > 13 {
                    // this is to appease the compiler for now, but globally an impossible
                    // state.
                    assert!(false);
                    unimplemented!()
                } else {
                    let r = insert_rec(sub, h, lvl + 1, k, v)?;
                    let e = SharedRef::new(Entry::SubNode(r));
                    Ok(node.replace_at(idx, e))
                }
            }
        }
    }
}

pub enum LookupRet<'a, K, V> {
    Found(&'a V),
    NotFound,
    ContinueIn(&'a Node<K, V>),
}

pub fn lookup_one<'a, K: PartialEq, V>(
    node: &'a Node<K, V>,
    h: &HashedKey,
    lvl: usize,
    k: &K,
) -> LookupRet<'a, K, V> {
    let level_hash = h.level_index(lvl);
    let idx = node.bitmap.get_index_sparse(level_hash);
    if idx.is_not_found() {
        LookupRet::NotFound
    } else {
        match &(node.get_child(idx)).as_ref() {
            &Entry::Leaf(content) => match content.find(*h, k) {
                None => LookupRet::NotFound,
                Some(v) => LookupRet::Found(v),
            },
            &Entry::SubNode(sub) => LookupRet::ContinueIn(sub),
        }
    }
}

// recursively try to remove a key with an expected equality value v
pub fn remove_eq_rec<K: PartialEq, V: PartialEq>(
    node: &Node<K, V>,
    h: &HashedKey,
    lvl: usize,
    k: &K,
    v: &V,
) -> Result<Option<Node<K, V>>, RemoveError> {
    let level_hash = h.level_index(lvl);
    let idx = node.bitmap.get_index_sparse(level_hash);
    if idx.is_not_found() {
        return Err(RemoveError::KeyNotFound);
    } else {
        match &(node.get_child(idx)).as_ref() {
            &Entry::Leaf(content) => {
                let new_content = content.remove_match(h, k, v)?;
                let new_ent = new_content.and_then(|x| Some(SharedRef::new(Entry::Leaf(x))));
                Ok(node.clear_or_replace_at(level_hash, new_ent))
            }
            &Entry::SubNode(sub) => match remove_eq_rec(sub, h, lvl + 1, k, v)? {
                None => Ok(node.clear_at(level_hash)),
                Some(newsub) => {
                    let e = Entry::SubNode(newsub);
                    Ok(Some(node.replace_at(idx, SharedRef::new(e))))
                }
            },
        }
    }
}

// recursively try to remove a key
pub fn remove_rec<K: PartialEq, V>(
    node: &Node<K, V>,
    h: &HashedKey,
    lvl: usize,
    k: &K,
) -> Result<Option<Node<K, V>>, RemoveError> {
    let level_hash = h.level_index(lvl);
    let idx = node.bitmap.get_index_sparse(level_hash);
    if idx.is_not_found() {
        return Err(RemoveError::KeyNotFound);
    } else {
        match &(node.get_child(idx)).as_ref() {
            &Entry::Leaf(content) => {
                let new_content = content.remove(h, k)?;
                let new_ent = new_content.and_then(|x| Some(SharedRef::new(Entry::Leaf(x))));
                Ok(node.clear_or_replace_at(level_hash, new_ent))
            }
            &Entry::SubNode(sub) => match remove_rec(sub, h, lvl + 1, k)? {
                None => Ok(node.clear_at(level_hash)),
                Some(newsub) => {
                    let e = Entry::SubNode(newsub);
                    Ok(Some(node.replace_at(idx, SharedRef::new(e))))
                }
            },
        }
    }
}

// recursively try to update a key.
//
// note, an update cannot create a new value, it can only delete or update an existing value.
pub fn update_rec<K: PartialEq + Clone, V, F, U>(
    node: &Node<K, V>,
    h: &HashedKey,
    lvl: usize,
    k: &K,
    f: F,
) -> Result<Option<Node<K, V>>, UpdateError<U>>
where
    F: FnOnce(&V) -> Result<Option<V>, U>,
{
    let level_hash = h.level_index(lvl);
    let idx = node.bitmap.get_index_sparse(level_hash);
    if idx.is_not_found() {
        return Err(UpdateError::KeyNotFound);
    } else {
        match &(node.get_child(idx)).as_ref() {
            &Entry::Leaf(content) => {
                let new_content = content.update(h, k, f)?;
                let new_ent = new_content.and_then(|x| Some(SharedRef::new(Entry::Leaf(x))));
                Ok(node.clear_or_replace_at(level_hash, new_ent))
            }
            &Entry::SubNode(sub) => match update_rec(sub, h, lvl + 1, k, f)? {
                None => Ok(node.clear_at(level_hash)),
                Some(newsub) => {
                    let e = Entry::SubNode(newsub);
                    Ok(Some(node.replace_at(idx, SharedRef::new(e))))
                }
            },
        }
    }
}

// recursively try to replace a key's value.
//
// note, an update cannot create a new value, it can only delete or update an existing value.
pub fn replace_rec<K: PartialEq + Clone, V: Clone>(
    node: &Node<K, V>,
    h: &HashedKey,
    lvl: usize,
    k: &K,
    v: V,
) -> Result<(Node<K, V>, V), ReplaceError> {
    let level_hash = h.level_index(lvl);
    let idx = node.bitmap.get_index_sparse(level_hash);
    if idx.is_not_found() {
        return Err(ReplaceError::KeyNotFound);
    } else {
        match &(node.get_child(idx)).as_ref() {
            &Entry::Leaf(content) => {
                let (new_content, oldv) = content.replace(k, v)?;
                let new_ent = SharedRef::new(Entry::Leaf(new_content));
                Ok((node.replace_at(idx, new_ent), oldv))
            }
            &Entry::SubNode(sub) => {
                let (newsub, oldv) = replace_rec(sub, h, lvl + 1, k, v)?;
                let e = Entry::SubNode(newsub);
                Ok((node.replace_at(idx, SharedRef::new(e)), oldv))
            }
        }
    }
}

pub fn size_rec<K, V>(node: &Node<K, V>) -> usize {
    let mut sum = 0;
    for c in node.children.iter() {
        match &c.as_ref() {
            &Entry::Leaf(ref content) => sum += content.len(),
            &Entry::SubNode(sub) => sum += size_rec(&sub),
        }
    }
    sum
}

//// debug
pub mod debug {
    use super::*;
    use std::cmp;

    pub fn depth_rec<K, V>(node: &Node<K, V>) -> usize {
        let mut max_depth = 0;
        for c in node.children.iter() {
            match &c.as_ref() {
                &Entry::Leaf(_) => {}
                &Entry::SubNode(sub) => {
                    let child_depth = depth_rec(&sub);
                    max_depth = cmp::max(max_depth, child_depth)
                }
            }
        }
        max_depth
    }
}
