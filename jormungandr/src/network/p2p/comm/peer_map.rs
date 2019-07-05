use super::PeerComms;
use crate::network::p2p::topology::NodeId;

use std::collections::{hash_map, HashMap};
use std::pin::Pin;
use std::ptr::NonNull;

pub struct PeerMap {
    map: HashMap<NodeId, Pin<Box<Node>>>,
    block_cursor: BlockFetchCursor,
}

unsafe impl Send for PeerMap {}

impl PeerMap {
    pub fn new() -> Self {
        PeerMap {
            map: HashMap::new(),
            block_cursor: BlockFetchCursor::Empty,
        }
    }

    pub fn entry<'a>(&'a mut self, id: NodeId) -> Option<Entry<'a>> {
        use std::collections::hash_map::Entry::*;

        match self.map.entry(id) {
            Vacant(_) => None,
            Occupied(entry) => Some(Entry {
                inner: entry,
                block_cursor: &mut self.block_cursor,
            }),
        }
    }

    pub fn peer_comms(&mut self, id: NodeId) -> Option<&mut PeerComms> {
        match self.map.get_mut(&id) {
            None => None,
            Some(pin) => Some(&mut pin.comms),
        }
    }

    pub fn ensure_peer_comms(&mut self, id: NodeId) -> &mut PeerComms {
        use std::collections::hash_map::Entry::*;

        let (node_ptr, last_needs_updating) = match self.map.entry(id) {
            Occupied(mut entry) => (entry.get_mut().as_mut().as_ptr(), false),
            Vacant(entry) => {
                let node = Box::pin(Node::new(id, PeerComms::new()));
                let node = entry.insert(node);
                (node.as_mut().as_ptr(), true)
            }
        };
        // TODO: update to edition 2018 so we can use NLLs
        if last_needs_updating {
            unsafe {
                self.push_last(id, node_ptr);
            }
        }
        unsafe { &mut (*node_ptr.as_ptr()).comms }
    }

    pub fn insert_peer(&mut self, id: NodeId, comms: PeerComms) {
        use std::collections::hash_map::Entry::*;

        let mut node = Box::pin(Node::new(id, comms));
        let (node_ptr, last_needs_updating) = match self.map.entry(id) {
            Occupied(mut entry) => {
                let (prev, next) = unsafe { entry.get_mut().unlink() };
                if next.is_none() {
                    // The old entry was the last,
                    // the cursor does not need updating.
                    // Just link with the previous node here.
                    node.prev = prev;
                    if let Some(mut prev) = prev {
                        unsafe {
                            prev.as_mut().next = Some(node.as_mut().as_ptr());
                        }
                    }
                }
                entry.insert(node);
                (entry.into_mut().as_mut().as_ptr(), next.is_some())
            }
            Vacant(entry) => (entry.insert(node).as_mut().as_ptr(), true),
        };
        if last_needs_updating {
            unsafe {
                self.push_last(id, node_ptr);
            }
        }
    }

    pub fn next_peer_for_block_fetch(&mut self) -> Option<(NodeId, &mut PeerComms)> {
        match self.block_cursor.next() {
            None => None,
            Some(id) => {
                let node = self.map.get_mut(&id).unwrap();
                let prev_id = match node.prev {
                    None => None,
                    Some(prev_ptr) => unsafe { Some(prev_ptr.as_ref().id) },
                };
                self.block_cursor.set_next(prev_id);
                Some((id, &mut node.comms))
            }
        }
    }

    unsafe fn push_last(&mut self, id: NodeId, mut node_ptr: NonNull<Node>) {
        if let Some(last_id) = self.block_cursor.last() {
            let last = self.map.get_mut(&last_id).unwrap();
            last.next = Some(node_ptr);
            let node = node_ptr.as_mut();
            node.prev = Some(NonNull::new_unchecked(last.as_mut().get_mut()));
            node.next = None;
        }
        self.block_cursor.set_last(id);
    }
}

// State for round-robin block fetching cursor.
enum BlockFetchCursor {
    // Placeholder when no entries exist in the map.
    Empty,
    Ids {
        // The ID of the last node in the order.
        last: NodeId,
        // Cursor for the next node to fetch blocks from.
        // If None, start from last.
        next_back: Option<NodeId>,
    },
}

impl BlockFetchCursor {
    fn is_last(&self, id: NodeId) -> bool {
        match self {
            BlockFetchCursor::Empty => false,
            BlockFetchCursor::Ids { last, .. } => *last == id,
        }
    }

    fn last(&self) -> Option<NodeId> {
        match self {
            BlockFetchCursor::Empty => None,
            BlockFetchCursor::Ids { last, .. } => Some(*last),
        }
    }

    fn next(&self) -> Option<NodeId> {
        match self {
            BlockFetchCursor::Empty => None,
            BlockFetchCursor::Ids { last, next_back } => next_back.or(Some(*last)),
        }
    }

    fn set_last(&mut self, last: NodeId) {
        *self = match self {
            BlockFetchCursor::Empty => BlockFetchCursor::Ids {
                last,
                next_back: None,
            },
            BlockFetchCursor::Ids { ref next_back, .. } => BlockFetchCursor::Ids {
                last,
                next_back: *next_back,
            },
        }
    }

    fn set_next(&mut self, next: Option<NodeId>) {
        match self {
            BlockFetchCursor::Empty => unreachable!("node key set in empty peer collection"),
            BlockFetchCursor::Ids { next_back, .. } => {
                *next_back = next;
            }
        }
    }
}

// Map node, pinned and linked through in linear order of recent use.
struct Node {
    // The node ID, duplicated in the value structure
    // to access when navigating "sideways" in the order.
    id: NodeId,
    // The structurally unpinned peer communications entry.
    comms: PeerComms,
    // Pointer to the previous node.
    prev: Option<NonNull<Node>>,
    // Pointer to the next node.
    next: Option<NonNull<Node>>,
}

unsafe impl Send for Node {}

impl Node {
    fn new(id: NodeId, comms: PeerComms) -> Self {
        Node {
            id,
            comms,
            prev: None,
            next: None,
        }
    }

    fn as_ptr(self: Pin<&mut Node>) -> NonNull<Node> {
        unsafe { NonNull::new_unchecked(self.get_mut()) }
    }

    // Require a mutable borrow on self because this modifies
    // adjacent nodes.
    unsafe fn unlink(&mut self) -> (Option<NonNull<Node>>, Option<NonNull<Node>>) {
        if let Some(mut prev) = self.prev {
            prev.as_mut().next = self.next;
        }
        if let Some(mut next) = self.next {
            next.as_mut().prev = self.prev;
        }
        (self.prev, self.next)
    }
}

pub struct Entry<'a> {
    inner: hash_map::OccupiedEntry<'a, NodeId, Pin<Box<Node>>>,
    block_cursor: &'a mut BlockFetchCursor,
}

impl<'a> Entry<'a> {
    pub fn comms(&mut self) -> &mut PeerComms {
        &mut self.inner.get_mut().comms
    }

    pub fn remove(mut self) {
        let id = *self.inner.key();
        let (prev, _) = unsafe { self.inner.get_mut().unlink() };
        if self.block_cursor.next() == Some(id) {
            let next = match prev {
                Some(prev) => unsafe { Some(prev.as_ref().id) },
                None => None,
            };
            self.block_cursor.set_next(next);
        }
        if self.block_cursor.is_last(id) {
            match prev {
                Some(prev) => self.block_cursor.set_last(unsafe { prev.as_ref().id }),
                None => {
                    *self.block_cursor = BlockFetchCursor::Empty;
                }
            }
        }
        self.inner.remove();
    }
}
