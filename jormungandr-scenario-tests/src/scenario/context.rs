use rand_chacha::ChaChaRng;
use rand_core::{RngCore, SeedableRng};
use std::{
    net::SocketAddr,
    sync::atomic::{self, AtomicU16},
};

/// scenario context with all the details to setup the necessary port number
/// a pseudo random number generator (and its original seed).
///
pub struct Context<RNG: RngCore + Sized> {
    rng: RNG,

    seed: [u8; 32],

    jormungandr: bawawa::Command,
    jcli: bawawa::Command,

    next_available_rest_port_number: AtomicU16,
    next_available_grpc_port_number: AtomicU16,
}

impl Context<ChaChaRng> {
    pub fn new(jormungandr: bawawa::Command, jcli: bawawa::Command) -> Self {
        let mut seed = [0; 32];
        rand::rngs::OsRng::new().unwrap().fill_bytes(&mut seed);
        let rng = ChaChaRng::from_seed(seed);

        Context {
            rng,
            seed,
            next_available_rest_port_number: AtomicU16::new(8_000),
            next_available_grpc_port_number: AtomicU16::new(12_000),
            jormungandr,
            jcli,
        }
    }
}

impl<RNG: RngCore> Context<RNG> {
    pub fn derive(&mut self) -> Self {
        unimplemented!()
    }

    pub fn jormungandr(&self) -> &bawawa::Command {
        &self.jormungandr
    }

    pub fn jcli(&self) -> &bawawa::Command {
        &self.jcli
    }

    pub fn rng_mut(&mut self) -> &mut RNG {
        &mut self.rng
    }

    pub fn generate_new_rest_listen_address(&mut self) -> SocketAddr {
        use std::net::{IpAddr, Ipv4Addr};

        let port_number = self
            .next_available_rest_port_number
            .fetch_add(1, atomic::Ordering::SeqCst);
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), port_number)
    }

    pub fn generate_new_grpc_public_address(&mut self) -> String {
        use std::net::{IpAddr, Ipv4Addr};

        let port_number = self
            .next_available_grpc_port_number
            .fetch_add(1, atomic::Ordering::SeqCst);

        let address = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));

        format!("/ip4/{}/tcp/{}", address, port_number)
    }

    /// retrieve the original seed of the pseudo random generator
    #[inline]
    pub fn seed(&self) -> &[u8; 32] {
        &self.seed
    }
}
