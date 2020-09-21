use crate::network::{
    client::ConnectHandle,
    p2p::{
        comm::{PeerComms, PeerInfo, PeerStats},
        Address,
    },
};
use linked_hash_map::LinkedHashMap;

pub struct PeerMap {
    map: LinkedHashMap<Address, PeerData>,
    capacity: usize,
}

#[derive(Default)]
struct PeerData {
    comms: PeerComms,
    stats: PeerStats,
    connecting: Option<ConnectHandle>,
}

pub enum CommStatus<'a> {
    Connecting(&'a mut PeerComms),
    Established(&'a mut PeerComms),
}

impl PeerData {
    fn new(comms: PeerComms) -> Self {
        PeerData {
            comms,
            stats: PeerStats::default(),
            connecting: None,
        }
    }

    fn update_comm_status(&mut self) -> CommStatus<'_> {
        if let Some(ref mut handle) = self.connecting {
            match handle.try_complete() {
                Ok(None) => return CommStatus::Connecting(&mut self.comms),
                Ok(Some(comms)) => {
                    self.connecting = None;
                    self.comms.update(comms);
                }
                Err(_) => {
                    self.connecting = None;
                }
            }
        }
        CommStatus::Established(&mut self.comms)
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

impl<'a> CommStatus<'a> {
    fn comms(self) -> &'a mut PeerComms {
        match self {
            CommStatus::Connecting(comms) => comms,
            CommStatus::Established(comms) => comms,
        }
    }
}

impl PeerMap {
    pub fn new(capacity: usize) -> Self {
        PeerMap {
            map: LinkedHashMap::new(),
            capacity,
        }
    }

    pub fn entry<'a>(&'a mut self, id: Address) -> Option<Entry<'a>> {
        use linked_hash_map::Entry::*;

        match self.map.entry(id) {
            Vacant(_) => None,
            Occupied(entry) => Some(Entry { inner: entry }),
        }
    }

    /// for clearing the peer map
    pub fn clear(&mut self) {
        self.map.clear()
    }

    pub fn refresh_peer(&mut self, id: &Address) -> Option<&mut PeerStats> {
        self.map.get_refresh(&id).map(|data| &mut data.stats)
    }

    pub fn peer_comms(&mut self, id: &Address) -> Option<&mut PeerComms> {
        self.map
            .get_mut(id)
            .map(|data| data.update_comm_status().comms())
    }

    fn ensure_peer(&mut self, id: Address) -> &mut PeerData {
        if !self.map.contains_key(&id) {
            self.evict_if_full();
        }
        self.map.entry(id).or_insert_with(Default::default)
    }

    pub fn server_comms(&mut self, id: Address) -> &mut PeerComms {
        self.ensure_peer(id).server_comms()
    }

    pub fn add_connecting(&mut self, id: Address, handle: ConnectHandle) -> &mut PeerComms {
        let data = self.ensure_peer(id);
        data.connecting = Some(handle);
        data.update_comm_status().comms()
    }

    pub fn remove_peer(&mut self, id: Address) -> Option<PeerComms> {
        self.map.remove(&id).map(|mut data| {
            // A bit tricky here: use PeerData::update_comm_status for the
            // side effect, then return the up-to-date member.
            data.update_comm_status();
            data.comms
        })
    }

    pub fn next_peer_for_block_fetch(&mut self) -> Option<(Address, &mut PeerComms)> {
        let mut iter = self.map.iter_mut();
        while let Some((id, data)) = iter.next_back() {
            match data.update_comm_status() {
                CommStatus::Established(comms) => return Some((id.clone(), comms)),
                CommStatus::Connecting(_) => {}
            }
        }
        None
    }

    pub fn infos(&self) -> Vec<PeerInfo> {
        use jormungandr_lib::multiaddr::multiaddr_to_socket_addr;

        self.map
            .iter()
            .map(|(id, data)| PeerInfo {
                addr: multiaddr_to_socket_addr(id.multi_address()),
                stats: data.stats.clone(),
            })
            .collect()
    }

    pub fn evict_clients(&mut self, num: usize) {
        for entry in self
            .map
            .entries()
            .filter(|entry| entry.get().comms.has_client_subscriptions())
            .take(num)
        {
            entry.remove();
        }
    }

    fn evict_if_full(&mut self) {
        if self.map.len() >= self.capacity {
            self.map.pop_front();
        }
    }
}

pub struct Entry<'a> {
    inner: linked_hash_map::OccupiedEntry<'a, Address, PeerData>,
}

impl<'a> Entry<'a> {
    pub fn address(&self) -> &Address {
        self.inner.key()
    }

    pub fn update_comm_status(&mut self) -> CommStatus<'_> {
        self.inner.get_mut().update_comm_status()
    }

    pub fn remove(self) {
        self.inner.remove();
    }
}
