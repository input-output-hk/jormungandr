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

        let node_ptr = match self.map.entry(id) {
            Occupied(mut entry) => entry.get_mut().as_mut().as_ptr(),
            Vacant(entry) => {
                let node = Box::pin(Node::new(id, PeerComms::new()));
                let node = entry.insert(node);
                let node_ptr = node.as_mut().as_ptr();
                unsafe {
                    // Add the node, but don't reset the cursor
                    // as the block subscription has not been established yet.
                    self.block_cursor.add_last(node_ptr);
                }
                node_ptr
            }
        };
        unsafe { &mut (*node_ptr.as_ptr()).comms }
    }

    pub fn insert_peer(&mut self, id: NodeId, comms: PeerComms) {
        use std::collections::hash_map::Entry::*;

        let mut node = Box::pin(Node::new(id, comms));
        let node_ptr = match self.map.entry(id) {
            Occupied(mut entry) => {
                unsafe {
                    let old_node = entry.get_mut();
                    let old_node_ptr = old_node.as_mut().as_ptr();
                    self.block_cursor.on_unlink_node(old_node_ptr);
                    old_node.unlink();
                }
                let node_ptr = node.as_mut().as_ptr();
                entry.insert(node);
                node_ptr
            }
            Vacant(entry) => entry.insert(node).as_mut().as_ptr(),
        };
        unsafe {
            self.block_cursor.push_last(node_ptr);
        }
    }

    pub fn next_peer_for_block_fetch(&mut self) -> Option<(NodeId, &mut PeerComms)> {
        unsafe {
            match self.block_cursor.next() {
                None => None,
                Some(node_ptr) => {
                    let node = node_ptr.as_ref();
                    Some((node.id, &mut (*node_ptr.as_ptr()).comms))
                }
            }
        }
    }

    pub fn bump_peer_for_block_fetch(&mut self, id: NodeId) {
        if let Some(node) = self.map.get_mut(&id) {
            unsafe {
                let node_ptr = node.as_mut().as_ptr();
                if !self.block_cursor.is_last(node_ptr) {
                    self.block_cursor.on_unlink_node(node_ptr);
                    node.unlink();
                    self.block_cursor.push_last(node_ptr);
                }
            }
        }
    }
}

// State for round-robin block fetching cursor.
enum BlockFetchCursor {
    // Placeholder when no entries exist in the map.
    Empty,
    Ptrs {
        // The last node in the fetch order.
        last: NonNull<Node>,
        // Cursor for the next node to fetch blocks from.
        // If None, start from last.
        next_back: Option<NonNull<Node>>,
    },
}

impl BlockFetchCursor {
    fn is_last(&self, node_ptr: NonNull<Node>) -> bool {
        match self {
            BlockFetchCursor::Empty => false,
            BlockFetchCursor::Ptrs { last, .. } => *last == node_ptr,
        }
    }

    unsafe fn next(&mut self) -> Option<NonNull<Node>> {
        match self {
            BlockFetchCursor::Empty => None,
            BlockFetchCursor::Ptrs {
                ref last,
                ref mut next_back,
            } => {
                let next_ptr = next_back.unwrap_or(*last);
                let next = next_ptr.as_ref();
                *next_back = next.prev;
                Some(next_ptr)
            }
        }
    }

    unsafe fn add_last(&mut self, mut node_ptr: NonNull<Node>) {
        debug_assert!(node_ptr.as_mut().prev.is_none());
        debug_assert!(node_ptr.as_mut().next.is_none());
        match self {
            BlockFetchCursor::Empty => {
                *self = BlockFetchCursor::Ptrs {
                    last: node_ptr,
                    next_back: None,
                };
            }
            BlockFetchCursor::Ptrs {
                last: ref mut last_ptr,
                ..
            } => {
                let last = last_ptr.as_mut();
                last.next = Some(node_ptr);
                let node = node_ptr.as_mut();
                node.prev = Some(*last_ptr);
                *last_ptr = node_ptr;
            }
        }
    }

    unsafe fn push_last(&mut self, mut node_ptr: NonNull<Node>) {
        debug_assert!(node_ptr.as_mut().prev.is_none());
        debug_assert!(node_ptr.as_mut().next.is_none());
        *self = match self {
            BlockFetchCursor::Empty => BlockFetchCursor::Ptrs {
                last: node_ptr,
                next_back: None,
            },
            BlockFetchCursor::Ptrs {
                last: ref mut last_ptr,
                ..
            } => {
                let last = last_ptr.as_mut();
                last.next = Some(node_ptr);
                let node = node_ptr.as_mut();
                node.prev = Some(*last_ptr);
                BlockFetchCursor::Ptrs {
                    last: node_ptr,
                    next_back: None,
                }
            }
        }
    }

    // This must be called before the unlink method is called on the node.
    unsafe fn on_unlink_node(&mut self, node_ptr: NonNull<Node>) {
        match self {
            BlockFetchCursor::Ptrs {
                ref mut last,
                ref mut next_back,
            } => {
                let node = node_ptr.as_ref();
                if *next_back == Some(node_ptr) {
                    *next_back = node.prev;
                }
                if *last == node_ptr {
                    match node.prev {
                        None => {
                            *self = BlockFetchCursor::Empty;
                        }
                        Some(prev_ptr) => {
                            *last = prev_ptr;
                        }
                    }
                }
            }
            BlockFetchCursor::Empty => {
                unreachable!("cursor is empty while a node is being removed")
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
    unsafe fn unlink(&mut self) {
        if let Some(mut prev_ptr) = self.prev {
            prev_ptr.as_mut().next = self.next;
            self.prev = None;
        }
        if let Some(mut next_ptr) = self.next {
            next_ptr.as_mut().prev = self.prev;
            self.next = None;
        }
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
        let node = self.inner.get_mut();
        let node_ptr = node.as_mut().as_ptr();
        unsafe {
            self.block_cursor.on_unlink_node(node_ptr);
            node.unlink();
        }
        self.inner.remove();
    }
}
