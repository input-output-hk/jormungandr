use chain_impl_mockchain::{
    fragment::{Fragment, FragmentId, FragmentRaw},
    transaction::Transaction,
    value::{Value, ValueError},
};
use chain_ser::mempack::ReadError;
use std::collections::{BTreeMap, HashMap};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Fragment has invalid structure")]
    InvalidStructure {
        #[source]
        #[from]
        source: ReadError,
    },

    #[error("this kind of fragment are not authorized through the mempool")]
    NotAuthorizedFragment,

    #[error("The transaction is not properly balanced")]
    NotProperlyBalanced {
        #[source]
        #[from]
        source: ValueError,
    },
}

pub struct Entry {
    fragment: FragmentRaw,
    score: u64,
    previous: *mut Entry,
    next: *mut Entry,
}

/// Maintain a mempool of Fragment (to add on the blockchain)
///
/// If this is on a simple node that is not a block minter/miner then
/// it does not make sense to use a large capacity size. However for
/// a block miner is is advised to use a rather large mempool.
///
/// the mempool allows to quickly search entries:
///
/// * by hash
/// * by fee
/// * by size
/// * by insertion order (first or last one)
///
/// Having this much flexibility of the entry selection allows to
/// optimise the block creation to include the most rewarding first
/// (fees) and then try to feel the gaps with the smallest entries
///
pub struct Mempool {
    by_hash: HashMap<FragmentId, Box<Entry>>,
    by_fee: BTreeMap<u64, HashMap<FragmentId, *mut Entry>>,
    by_size: BTreeMap<usize, HashMap<FragmentId, *mut Entry>>,
    head: *mut Entry,
    tail: *mut Entry,
    cap: usize,
}

impl Mempool {
    const DEFAULT_CAPACITY: usize = 10_000;

    pub fn new(cap: usize) -> Self {
        Self {
            cap,
            by_hash: HashMap::new(),
            by_fee: BTreeMap::new(),
            by_size: BTreeMap::new(),
            head: std::ptr::null_mut(),
            tail: std::ptr::null_mut(),
        }
    }

    pub fn contains(&self, fragment_id: &FragmentId) -> bool {
        self.by_hash.contains_key(fragment_id)
    }

    /// add the given fragment, return the fragment that was already in
    /// if the capacity is reached. The removed fragment will be the oldest
    /// added fragment of the pool
    ///
    /// if the fragment is already in the mempool, nothing is done
    pub fn push(&mut self, fragment: FragmentRaw) -> Result<Option<FragmentId>, Error> {
        let entry = Entry::new(fragment)?;
        let id = entry.id();

        if self.contains(&id) {
            return Ok(None);
        }

        let oldest = if self.len() == self.capacity() {
            self.pop_oldest()
        } else {
            None
        };

        let score = entry.score;
        let size = entry.fragment.size_bytes_plus_size();
        let mut entry = Box::new(entry);
        let ptr: *mut Entry = &mut *entry;

        self.by_hash.insert(id, entry);
        self.by_fee.entry(score).or_default().insert(id, ptr);
        self.by_size.entry(size).or_default().insert(id, ptr);

        self.attach(ptr);

        Ok(oldest)
    }

    fn pop_oldest(&mut self) -> Option<FragmentId> {
        let id = unsafe { self.tail.as_mut() }?.id();

        self.remove(&id)?;

        Some(id)
    }

    /// get the entry that has been in the mempool for the longest time
    pub fn peek_oldest(&self) -> Option<&Entry> {
        unsafe { self.tail.as_ref() }
    }

    /// get (one of) the entry has has the smallest size
    ///
    /// this is useful for algorithm trying to fit in as many entries in the
    /// most compact amount of space
    pub fn peek_smallest(&self) -> Option<&Entry> {
        let entry = self.by_size.values().next_back()?.values().next()?;

        unsafe { entry.as_ref() }
    }

    /// get (one of) the largest entry
    pub fn peek_largest(&self) -> Option<&Entry> {
        let entry = self.by_size.values().next()?.values().next()?;

        unsafe { entry.as_ref() }
    }

    /// get (one of) the entry that pay the most fees
    pub fn peek_most_fee(&self) -> Option<&Entry> {
        let entry = self.by_fee.values().next()?.values().next()?;

        unsafe { entry.as_ref() }
    }

    /// remove an entry from the mempool
    ///
    /// this could be because it is selected to go on chain or a block
    /// already referenced it.
    ///
    pub fn remove(&mut self, id: &FragmentId) -> Option<Box<Entry>> {
        if let Some(mut entry) = self.by_hash.remove(id) {
            let id = entry.id();
            let score = entry.score;
            let size = entry.fragment.size_bytes_plus_size();

            self.by_fee.entry(score).and_modify(|e| {
                e.remove(&id);
            });
            self.by_size.entry(size).and_modify(|e| {
                e.remove(&id);
            });

            let ptr: *mut Entry = &mut *entry;
            if ptr == self.tail {
                self.tail = entry.previous;
            }
            if ptr == self.head {
                self.head = entry.next;
            }
            entry.detach();

            Some(entry)
        } else {
            None
        }
    }

    /// get the total number of entries in the mempool (never
    /// larger than the capacity)
    pub fn len(&self) -> usize {
        self.by_hash.len()
    }

    /// tell if the mempool is empty
    pub fn is_empty(&self) -> bool {
        self.by_hash.is_empty()
    }

    /// retrieve the current capacity of the Mempool
    pub fn capacity(&self) -> usize {
        self.cap
    }

    /// refresh the capacity of the Mempool
    ///
    /// If some fragments are removed because of this, their `FragmentId` will
    /// be returned so events can be associated to that.
    ///
    pub fn resize(&mut self, size: usize) -> Vec<FragmentId> {
        // create the Vec with the initial capacity for the detected number
        // of entry to remove from the Mempool due to resizing
        // if the num to remove is 0 then nothing is changed.

        let mut left_to_remove = self.cap.wrapping_sub(size);
        let mut removed = Vec::with_capacity(left_to_remove);

        while left_to_remove > 0 {
            if let Some(id) = self.pop_oldest() {
                removed.push(id);
            } else {
                break;
            }
            left_to_remove -= 1;
        }

        removed
    }

    fn attach(&mut self, entry: *mut Entry) {
        unsafe {
            (*entry).next = self.head;
        }
        if let Some(ptr) = unsafe { self.head.as_mut() } {
            ptr.previous = entry;
        } else {
            self.tail = entry;
        }

        self.head = entry;
    }
}

impl Default for Mempool {
    fn default() -> Self {
        Self::new(Self::DEFAULT_CAPACITY)
    }
}

impl Entry {
    fn new(fragment: FragmentRaw) -> Result<Self, Error> {
        let decoded = Fragment::from_raw(&fragment)?;
        check(&decoded)?;

        Ok(Self {
            fragment,
            score: score(&decoded)?,
            next: std::ptr::null_mut(),
            previous: std::ptr::null_mut(),
        })
    }

    pub fn id(&self) -> FragmentId {
        self.fragment.id()
    }

    pub fn size(&self) -> usize {
        self.fragment.size_bytes_plus_size()
    }

    pub fn fee(&self) -> u64 {
        self.score
    }

    fn detach(&mut self) {
        if let Some(previous) = unsafe { self.previous.as_mut() } {
            previous.next = self.next;
        }
        if let Some(next) = unsafe { self.next.as_mut() } {
            next.previous = self.previous;
        }
    }
}

fn check(fragment: &Fragment) -> Result<(), Error> {
    let reject_not_authorized = Err(Error::NotAuthorizedFragment);

    match fragment {
        Fragment::Initial(_) => reject_not_authorized,
        Fragment::OldUtxoDeclaration(_) => reject_not_authorized,
        Fragment::UpdateProposal(_) => reject_not_authorized,
        Fragment::UpdateVote(_) => reject_not_authorized,
        Fragment::Transaction(_t) => Ok(()),
        Fragment::OwnerStakeDelegation(_t) => Ok(()),
        Fragment::StakeDelegation(_t) => Ok(()),
        Fragment::PoolRegistration(_t) => Ok(()),
        Fragment::PoolRetirement(_t) => (Ok(())),
        Fragment::PoolUpdate(_t) => Ok(()),
        Fragment::VotePlan(_t) => Ok(()),
        Fragment::VoteCast(_t) => Ok(()),
        Fragment::VoteTally(_t) => Ok(()),
    }
}

fn fee<P>(t: &Transaction<P>) -> Result<Value, Error> {
    let input = t.total_input()?;
    let output = t.total_output()?;

    Ok(output.checked_sub(input)?)
}

fn score(fragment: &Fragment) -> Result<u64, Error> {
    let fee = match fragment {
        Fragment::Initial(_) => 0,
        Fragment::OldUtxoDeclaration(_) => 0,
        Fragment::UpdateProposal(_) => 0,
        Fragment::UpdateVote(_) => 0,
        Fragment::Transaction(t) => fee(t)?.0,
        Fragment::OwnerStakeDelegation(t) => fee(t)?.0,
        Fragment::StakeDelegation(t) => fee(t)?.0,
        Fragment::PoolRegistration(t) => fee(t)?.0,
        Fragment::PoolRetirement(t) => fee(t)?.0,
        Fragment::PoolUpdate(t) => fee(t)?.0,
        Fragment::VotePlan(t) => fee(t)?.0,
        Fragment::VoteCast(t) => fee(t)?.0,
        Fragment::VoteTally(t) => fee(t)?.0,
    };

    Ok(fee)
}
