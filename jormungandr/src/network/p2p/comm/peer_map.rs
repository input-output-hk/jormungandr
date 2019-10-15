use super::super::topology::NodeId;
use super::{PeerComms, PeerStats};
use crate::network::client::ConnectHandle;

use linked_hash_map::LinkedHashMap;

pub struct PeerMap {
    map: LinkedHashMap<NodeId, PeerData>,
    capacity: usize,
}

#[derive(Default)]
struct PeerData {
    comms: PeerComms,
    stats: PeerStats,
    connecting: Option<ConnectHandle>,
}

impl PeerData {
    fn with_comms(comms: PeerComms) -> Self {
        PeerData {
            comms,
            stats: PeerStats::default(),
            connecting: None,
        }
    }

    fn updated_comms(&mut self) -> &mut PeerComms {
        if let Some(ref mut handle) = self.connecting {
            match handle.try_complete() {
                Ok(None) => {}
                Ok(Some(comms)) => {
                    self.connecting = None;
                    self.comms.update(comms);
                }
                Err(_) => {
                    self.connecting = None;
                }
            }
        }
        &mut self.comms
    }

    fn server_comms(&mut self) -> &mut PeerComms {
        // This method is called when a subscription request is received
        // by the server, normally at the beginning of the peer connecting
        // as a client. Cancel client connection if it is pending.
        self.connecting = None;
        self.comms.clear_pending();
        &mut self.comms
    }
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

    pub fn refresh_peer(&mut self, id: NodeId) -> Option<&mut PeerStats> {
        self.map.get_refresh(&id).map(|data| &mut data.stats)
    }

    pub fn peer_comms(&mut self, id: NodeId) -> Option<&mut PeerComms> {
        self.map.get_mut(&id).map(PeerData::updated_comms)
    }

    fn ensure_peer(&mut self, id: NodeId) -> &mut PeerData {
        if !self.map.contains_key(&id) {
            self.evict_if_full();
        }
        self.map.entry(id).or_insert_with(Default::default)
    }

    pub fn server_comms(&mut self, id: NodeId) -> &mut PeerComms {
        self.ensure_peer(id).server_comms()
    }

    pub fn insert_peer(&mut self, id: NodeId, comms: PeerComms) {
        self.evict_if_full();
        let data = PeerData::with_comms(comms);
        self.map.insert(id, data);
    }

    pub fn add_connecting(&mut self, id: NodeId, handle: ConnectHandle) -> &mut PeerComms {
        let data = self.ensure_peer(id);
        data.connecting = Some(handle);
        data.updated_comms()
    }

    pub fn remove_peer(&mut self, id: NodeId) -> Option<PeerComms> {
        self.map.remove(&id).map(|mut data| {
            // A bit tricky here: use PeerData::updated_comms for the
            // side effect, then return the up-to-date member.
            data.updated_comms();
            data.comms
        })
    }

    pub fn next_peer_for_block_fetch(&mut self) -> Option<(NodeId, &mut PeerComms)> {
        self.map
            .iter_mut()
            .next_back()
            .map(|(&id, data)| (id, data.updated_comms()))
    }

    pub fn stats(&self) -> Vec<(NodeId, PeerStats)> {
        self.map
            .iter()
            .map(|(&id, data)| (id, data.stats.clone()))
            .collect()
    }

    fn evict_if_full(&mut self) {
        if self.map.len() >= self.capacity {
            self.map.pop_front();
        }
    }
}

pub struct Entry<'a> {
    inner: linked_hash_map::OccupiedEntry<'a, NodeId, PeerData>,
}

impl<'a> Entry<'a> {
    pub fn updated_comms(&mut self) -> &mut PeerComms {
        self.inner.get_mut().updated_comms()
    }

    pub fn stats(&mut self) -> &mut PeerStats {
        &mut self.inner.get_mut().stats
    }

    pub fn remove(self) {
        self.inner.remove();
    }
}
