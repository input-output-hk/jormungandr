use super::PeerComms;
use crate::network::p2p::topology::NodeId;

use linked_hash_map::LinkedHashMap;

pub struct PeerMap {
    map: LinkedHashMap<NodeId, PeerComms>,
    capacity: usize,
}

impl PeerMap {
    pub fn new(capacity: usize) -> Self {
        PeerMap {
            map: LinkedHashMap::new(),
            capacity,
        }
    }

    pub fn entry<'a>(&'a mut self, id: NodeId) -> Option<Entry<'a>> {
        use linked_hash_map::Entry::*;

        match self.map.entry(id) {
            Vacant(_) => None,
            Occupied(entry) => Some(Entry { inner: entry }),
        }
    }

    pub fn peer_comms(&mut self, id: NodeId) -> Option<&mut PeerComms> {
        self.map.get_mut(&id)
    }

    pub fn ensure_peer_comms(&mut self, id: NodeId) -> &mut PeerComms {
        if !self.map.contains_key(&id) {
            self.insert_peer(id, PeerComms::new());
        }
        self.map.get_mut(&id).unwrap()
    }

    pub fn insert_peer(&mut self, id: NodeId, comms: PeerComms) {
        self.evict_if_full();
        self.map.insert(id, comms);
    }

    pub fn remove_peer(&mut self, id: NodeId) -> Option<PeerComms> {
        self.map.remove(&id)
    }

    pub fn next_peer_for_block_fetch(&mut self) -> Option<(NodeId, &mut PeerComms)> {
        self.map
            .iter_mut()
            .next_back()
            .map(|(&id, comms)| (id, comms))
    }

    pub fn bump_peer_for_block_fetch(&mut self, id: NodeId) {
        // It's OK for the entry to be missing because it might have been
        // removed by the time peer's traffic is processed.
        let _ = self.map.get_refresh(&id);
    }

    fn evict_if_full(&mut self) {
        if self.map.len() >= self.capacity {
            self.map.pop_front();
        }
    }
}

pub struct Entry<'a> {
    inner: linked_hash_map::OccupiedEntry<'a, NodeId, PeerComms>,
}

impl<'a> Entry<'a> {
    pub fn comms(&mut self) -> &mut PeerComms {
        self.inner.get_mut()
    }

    pub fn remove(self) {
        self.inner.remove();
    }
}
