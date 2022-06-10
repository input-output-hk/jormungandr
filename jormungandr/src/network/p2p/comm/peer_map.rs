use crate::{
    metrics::{Metrics, MetricsBackend},
    network::{
        client::ConnectHandle,
        p2p::comm::{Address, PeerComms, PeerInfo, PeerStats},
        security_params::NONCE_LEN,
    },
    topology::NodeId,
};
use chain_network::error::{Code as ErrorCode, Error as NetworkError};
use linked_hash_map::LinkedHashMap;
use lru::LruCache;
use rand::Rng;

/// Peer authentication is checked during the handshake. For client connections, we simply
/// do not add a peer to the map if the authentication fails.
/// On the server side instead, we need to keep track of in progress handshakes until we have
/// authenticated reliably a node by its id.
/// FIXME: In addition, until a better solution is implemented, a correspondence between addresses
/// and ids is needed to handle subscriptions. However, this is subject to ip spoofing
/// attacks and should not be used in an open network.
pub struct ClientAuth {
    // Limit the number of in progress handshakes to prevent SYN flood-like resource exhaustion attacks
    in_progress: LruCache<Address, [u8; NONCE_LEN]>,
    established: LinkedHashMap<Address, NodeId>,
}

impl Default for ClientAuth {
    fn default() -> Self {
        Self {
            in_progress: LruCache::new(ClientAuth::DEFAULT_OPEN_HANDSHAKED_LIMIT),
            established: LinkedHashMap::new(),
        }
    }
}

impl ClientAuth {
    const DEFAULT_OPEN_HANDSHAKED_LIMIT: usize = 128;

    pub fn generate_auth_nonce(&mut self, addr: Address) -> [u8; NONCE_LEN] {
        let mut nonce = [0u8; NONCE_LEN];
        rand::thread_rng().fill(&mut nonce[..]);

        self.established.remove(&addr);
        self.in_progress.put(addr, nonce);
        nonce
    }

    pub fn complete_handshake<F>(
        &mut self,
        addr: Address,
        id: NodeId,
        verify: F,
    ) -> Result<(), NetworkError>
    where
        F: FnOnce([u8; NONCE_LEN]) -> Result<(), NetworkError>,
    {
        if let Some(nonce) = self.in_progress.pop(&addr) {
            verify(nonce)?;
            self.established.insert(addr, id);
            Ok(())
        } else {
            Err(NetworkError::new(
                ErrorCode::FailedPrecondition,
                "nonce missing, perform handshake first",
            ))
        }
    }

    pub fn remove(&mut self, addr: Address) -> Option<NodeId> {
        self.established.remove(&addr)
    }

    pub fn client_id(&self, addr: Address) -> Option<&NodeId> {
        self.established.get(&addr)
    }
}

pub struct PeerMap {
    map: LinkedHashMap<NodeId, PeerData>,
    client_auth: ClientAuth,
    capacity: usize,
    stats_counter: Metrics,
}

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
    fn new(remote_addr: Address) -> Self {
        Self {
            comms: PeerComms::new(remote_addr),
            stats: Default::default(),
            connecting: Default::default(),
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
        // This method is called when a handshake or subscription request is
        // received by the server, normally after when the peer connects
        // as a client. Cancel client connection if it is pending.
        //
        // TODO: remove client-server connection resolution logic
        // since we tabulate peer entries per address rather than node ID.
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
    pub fn new(capacity: usize, stats_counter: Metrics) -> Self {
        PeerMap {
            map: LinkedHashMap::new(),
            client_auth: ClientAuth::default(),
            capacity,
            stats_counter,
        }
    }

    pub fn entry(&mut self, id: NodeId) -> Option<Entry<'_>> {
        use linked_hash_map::Entry::*;

        match self.map.entry(id) {
            Vacant(_) => None,
            Occupied(entry) => {
                let auth_info = self
                    .client_auth
                    .established
                    .entry(entry.get().comms.remote_addr);
                Some(Entry {
                    inner: entry,
                    auth_info,
                    stats_counter: &self.stats_counter,
                })
            }
        }
    }

    /// for clearing the peer map
    pub fn clear(&mut self) {
        self.stats_counter.sub_peer_connected_cnt(self.map.len());
        self.map.clear()
    }

    pub fn refresh_peer(&mut self, id: &NodeId) -> Option<&mut PeerStats> {
        self.map.get_refresh(id).map(|data| &mut data.stats)
    }

    pub fn peer_comms(&mut self, id: &NodeId) -> Option<&mut PeerComms> {
        self.map
            .get_mut(id)
            .map(|data| data.update_comm_status().comms())
    }

    fn ensure_peer(&mut self, id: NodeId, remote_addr: Address) -> &mut PeerData {
        if !self.map.contains_key(&id) {
            self.evict_if_full();
            self.stats_counter.add_peer_connected_cnt(1);
        }
        self.map
            .entry(id)
            .or_insert_with(|| PeerData::new(remote_addr))
    }

    pub fn server_comms(&mut self, id: &NodeId) -> Option<&mut PeerComms> {
        self.map.get_mut(id).map(|peer| peer.server_comms())
    }

    pub fn generate_auth_nonce(&mut self, addr: Address) -> [u8; NONCE_LEN] {
        self.client_auth.generate_auth_nonce(addr)
    }

    pub fn client_id(&self, peer_addr: Address) -> Option<&NodeId> {
        self.client_auth.client_id(peer_addr)
    }

    pub fn complete_handshake<F>(
        &mut self,
        addr: Address,
        id: NodeId,
        verify: F,
    ) -> Result<(), NetworkError>
    where
        F: FnOnce([u8; NONCE_LEN]) -> Result<(), NetworkError>,
    {
        self.client_auth.complete_handshake(addr, id, verify)?;
        self.add_client(id, addr);
        Ok(())
    }

    // This is called when connecting as a client to another node
    pub fn add_connecting(
        &mut self,
        id: NodeId,
        remote_addr: Address,
        handle: ConnectHandle,
    ) -> &mut PeerComms {
        let data = self.ensure_peer(id, remote_addr);
        data.connecting = Some(handle);
        data.update_comm_status().comms()
    }

    // This is called when accepting client connections as a server
    pub fn add_client(&mut self, id: NodeId, remote_addr: Address) -> &mut PeerComms {
        let data = self.ensure_peer(id, remote_addr);
        data.update_comm_status().comms()
    }

    pub fn remove_peer(&mut self, id: &NodeId) -> Option<PeerComms> {
        self.map.remove(id).map(|mut data| {
            // A bit tricky here: use PeerData::update_comm_status for the
            // side effect, then return the up-to-date member.
            data.update_comm_status();
            self.stats_counter.sub_peer_connected_cnt(1);
            self.client_auth.remove(data.comms.remote_addr());
            data.comms
        })
    }

    pub fn next_peer_for_block_fetch(&mut self) -> Option<(NodeId, &mut PeerComms)> {
        let mut iter = self.map.iter_mut();
        while let Some((id, data)) = iter.next_back() {
            match data.update_comm_status() {
                CommStatus::Established(comms) => return Some((*id, comms)),
                CommStatus::Connecting(_) => {}
            };
        }
        None
    }

    pub fn infos(&self) -> Vec<PeerInfo> {
        self.map
            .iter()
            .map(|(&id, data)| PeerInfo {
                id,
                addr: None,
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
            if let Some((_, v)) = self.map.pop_front() {
                self.stats_counter.sub_peer_connected_cnt(1);
                self.client_auth.remove(v.comms.remote_addr());
            }
        }
    }
}

pub struct Entry<'a> {
    inner: linked_hash_map::OccupiedEntry<'a, NodeId, PeerData>,
    auth_info: linked_hash_map::Entry<'a, Address, NodeId>,
    stats_counter: &'a Metrics,
}

impl<'a> Entry<'a> {
    pub fn update_comm_status(&mut self) -> CommStatus<'_> {
        self.inner.get_mut().update_comm_status()
    }

    pub fn remove(self) {
        use linked_hash_map::Entry::*;
        self.inner.remove();
        self.stats_counter.sub_peer_connected_cnt(1);
        if let Occupied(entry) = self.auth_info {
            entry.remove();
        }
    }
}
