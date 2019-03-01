use super::hash::HashedKey;
use super::helper::*;
use super::operation::*;
use super::sharedref::SharedRef;
use std::slice;

pub struct KV<K, V>(K, V);

impl<K, V> KV<K, V> {
    pub fn new(k: K, v: V) -> Self {
        KV(k, v)
    }
    pub fn get_key(&self) -> &K {
        &self.0
    }

    pub fn get_value(&self) -> &V {
        &self.1
    }
}

pub enum SmallVec<T> {
    One(T),
    //Two(T, T),
    Many(Vec<T>),
}

/// Leaf content is usually one key-value pair,
/// but can contains multiples pair when having a collision.
///
/// All the key held here have the same hash
pub struct LeafContent<K, V> {
    pub(crate) hashed: HashedKey,
    pub(crate) content: SmallVec<SharedRef<KV<K, V>>>,
}

impl<K, V> LeafContent<K, V> {
    pub fn len(&self) -> usize {
        match self.content {
            SmallVec::One(_) => 1,
            SmallVec::Many(ref v) => v.len(),
        }
    }
}

impl<K: PartialEq, V> LeafContent<K, V> {
    pub fn single(h: HashedKey, kv: SharedRef<KV<K, V>>) -> Self {
        LeafContent {
            hashed: h,
            content: SmallVec::One(kv),
        }
    }

    pub fn add(&self, kv: SharedRef<KV<K, V>>) -> Result<Self, InsertError> {
        // check for duplicated key
        match self.content {
            SmallVec::One(ref fkv) => {
                if kv.get_key() == fkv.get_key() {
                    return Err(InsertError::EntryExists);
                };
                let v = vec![SharedRef::clone(fkv), kv];
                Ok(LeafContent {
                    hashed: self.hashed,
                    content: SmallVec::Many(v),
                })
            }
            SmallVec::Many(ref content) => {
                for fkv in content.iter() {
                    if kv.get_key() == fkv.get_key() {
                        return Err(InsertError::EntryExists);
                    }
                }
                let mut v = Vec::with_capacity(content.len() + 1);
                v.extend_from_slice(&content[..]);
                v.push(kv);
                Ok(LeafContent {
                    hashed: self.hashed,
                    content: SmallVec::Many(v),
                })
            }
        }
    }

    pub fn find(&self, h: HashedKey, k: &K) -> Option<&V> {
        if self.hashed == h {
            // looks in all the keys for a match
            match self.content {
                SmallVec::One(ref fkv) => {
                    if k == fkv.get_key() {
                        return Some(fkv.get_value());
                    }
                    None
                }
                SmallVec::Many(ref v) => {
                    for fkv in v.iter() {
                        if k == fkv.get_key() {
                            return Some(fkv.get_value());
                        }
                    }
                    None
                }
            }
        } else {
            None
        }
    }
}

pub enum LeafIterator<'a, K, V> {
    One(bool, &'a SharedRef<KV<K, V>>),
    Many(slice::Iter<'a, SharedRef<KV<K, V>>>),
}

impl<'a, K, V> LeafContent<K, V> {
    pub fn iter(&'a self) -> LeafIterator<'a, K, V> {
        match self.content {
            SmallVec::Many(ref content) => LeafIterator::Many(content.iter()),
            SmallVec::One(ref kvs) => LeafIterator::One(false, kvs),
        }
    }
}

impl<'a, K, V> Iterator for LeafIterator<'a, K, V> {
    type Item = &'a SharedRef<KV<K, V>>;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            LeafIterator::Many(ref mut c) => c.next(),
            LeafIterator::One(ref mut consumed, o) => {
                if *consumed {
                    None
                } else {
                    *consumed = true;
                    Some(o)
                }
            }
        }
    }
}

impl<K: PartialEq + Clone, V> LeafContent<K, V> {
    pub fn update<F>(&self, h: &HashedKey, k: &K, f: F) -> Result<Option<Self>, UpdateError>
    where
        F: FnOnce(&V) -> Result<Option<V>, UpdateError>,
    {
        if self.hashed != *h {
            return Err(UpdateError::KeyNotFound);
        }
        match self.content {
            SmallVec::One(ref fkv) => {
                if k != fkv.get_key() {
                    return Err(UpdateError::KeyNotFound);
                }
                match f(fkv.get_value())? {
                    None => Ok(None),
                    Some(newv) => {
                        let newkv = KV::new(k.clone(), newv);
                        let newcontent = LeafContent {
                            hashed: self.hashed,
                            content: SmallVec::One(SharedRef::new(newkv)),
                        };
                        Ok(Some(newcontent))
                    }
                }
            }
            SmallVec::Many(ref content) => {
                assert!(content.len() > 1);
                // looks in all the keys for a match
                let mut found = None;
                for (i, fkv) in content.iter().enumerate() {
                    if k == fkv.get_key() {
                        found = Some(i);
                        break;
                    }
                }
                match found {
                    None => Err(UpdateError::KeyNotFound),
                    Some(pos) => {
                        // content == 1 is handled by SmallVec::One
                        match f(content[pos].get_value())? {
                            None => {
                                // trigger deletion
                                if content.len() == 2 {
                                    let to_keep = if pos == 0 {
                                        SharedRef::clone(&content[1])
                                    } else {
                                        SharedRef::clone(&content[0])
                                    };
                                    Ok(Some(LeafContent {
                                        hashed: self.hashed,
                                        content: SmallVec::One(to_keep),
                                    }))
                                } else {
                                    let mut newv = content.clone();
                                    newv.remove(pos);
                                    Ok(Some(LeafContent {
                                        hashed: self.hashed,
                                        content: SmallVec::Many(newv),
                                    }))
                                }
                            }
                            Some(newv) => {
                                // update vector at position
                                let newkv = KV::new(k.clone(), newv);
                                let new_array =
                                    clone_array_and_set_at_pos(content, SharedRef::new(newkv), pos);
                                let newcontent = LeafContent {
                                    hashed: self.hashed,
                                    content: SmallVec::Many(new_array),
                                };
                                Ok(Some(newcontent))
                            }
                        }
                    }
                }
            }
        }
    }
}

impl<K: PartialEq, V> LeafContent<K, V> {
    pub fn remove(&self, h: &HashedKey, k: &K) -> Result<Option<Self>, RemoveError> {
        if self.hashed != *h {
            return Err(RemoveError::KeyNotFound);
        }
        match self.content {
            SmallVec::One(ref fkv) => {
                if k != fkv.get_key() {
                    return Err(RemoveError::KeyNotFound);
                }
                return Ok(None);
            }
            SmallVec::Many(ref content) => {
                assert!(content.len() > 1);
                // looks in all the keys for a match
                let mut found = None;
                for (i, fkv) in content.iter().enumerate() {
                    if k == fkv.get_key() {
                        found = Some(i);
                        break;
                    }
                }
                match found {
                    None => Err(RemoveError::KeyNotFound),
                    Some(pos) => {
                        if content.len() == 1 {
                            Ok(None)
                        } else if content.len() == 2 {
                            let to_keep = if pos == 0 {
                                SharedRef::clone(&content[1])
                            } else {
                                SharedRef::clone(&content[0])
                            };
                            Ok(Some(LeafContent {
                                hashed: self.hashed,
                                content: SmallVec::One(to_keep),
                            }))
                        } else {
                            let mut newv = content.clone();
                            newv.remove(pos);
                            Ok(Some(LeafContent {
                                hashed: self.hashed,
                                content: SmallVec::Many(newv),
                            }))
                        }
                    }
                }
            }
        }
    }
}

impl<K: PartialEq, V: PartialEq> LeafContent<K, V> {
    pub fn remove_match(&self, h: &HashedKey, k: &K, v: &V) -> Result<Option<Self>, RemoveError> {
        if self.hashed != *h {
            return Err(RemoveError::KeyNotFound);
        }

        match self.content {
            SmallVec::One(ref fkv) => {
                if k != fkv.get_key() {
                    return Err(RemoveError::KeyNotFound);
                }
                if v != fkv.get_value() {
                    return Err(RemoveError::ValueNotMatching);
                }
                return Ok(None);
            }
            SmallVec::Many(ref content) => {
                assert!(content.len() > 1);
                // looks in all the keys for a match
                let mut found = None;
                for (i, fkv) in content.iter().enumerate() {
                    if k == fkv.get_key() {
                        found = Some(i);
                        break;
                    }
                }
                match found {
                    None => Err(RemoveError::KeyNotFound),
                    Some(pos) => {
                        if content[pos].get_value() != v {
                            return Err(RemoveError::ValueNotMatching);
                        }

                        if content.len() == 1 {
                            Ok(None)
                        } else if content.len() == 2 {
                            let to_keep = if pos == 0 {
                                SharedRef::clone(&content[1])
                            } else {
                                SharedRef::clone(&content[0])
                            };
                            Ok(Some(LeafContent {
                                hashed: self.hashed,
                                content: SmallVec::One(to_keep),
                            }))
                        } else {
                            let mut newv = content.clone();
                            newv.remove(pos);
                            Ok(Some(LeafContent {
                                hashed: self.hashed,
                                content: SmallVec::Many(newv),
                            }))
                        }
                    }
                }
            }
        }
    }
}
