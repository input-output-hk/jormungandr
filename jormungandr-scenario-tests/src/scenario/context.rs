use rand_chacha::ChaChaRng;
use rand_core::{RngCore, SeedableRng};
use std::{
    fmt::{self, Display, Formatter},
    net::SocketAddr,
    str::FromStr,
    sync::{
        atomic::{self, AtomicU16},
        Arc,
    },
};

pub type ContextChaCha = Context<ChaChaRng>;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Seed([u8; 32]);

/// scenario context with all the details to setup the necessary port number
/// a pseudo random number generator (and its original seed).
///
pub struct Context<RNG: RngCore + Sized> {
    rng: RNG,

    seed: Seed,

    jormungandr: bawawa::Command,
    jcli: bawawa::Command,

    next_available_rest_port_number: Arc<AtomicU16>,
    next_available_grpc_port_number: Arc<AtomicU16>,
}

impl Seed {
    fn zero() -> Self {
        Seed([0; 32])
    }

    pub fn generate<RNG: RngCore>(mut rng: RNG) -> Self {
        let mut seed = Seed::zero();
        rng.fill_bytes(&mut seed.0);
        seed
    }
}

impl Context<ChaChaRng> {
    pub fn new(seed: Seed, jormungandr: bawawa::Command, jcli: bawawa::Command) -> Self {
        let rng = ChaChaRng::from_seed(seed.0);

        Context {
            rng,
            seed,
            next_available_rest_port_number: Arc::new(AtomicU16::new(8_000)),
            next_available_grpc_port_number: Arc::new(AtomicU16::new(12_000)),
            jormungandr,
            jcli,
        }
    }

    /// derive the Context into a new context, seeding a new RNG from the original
    /// Context (so reproducibility is still available).
    pub fn derive(&mut self) -> Self {
        let seed = Seed::generate(self.rng_mut());
        let rng = ChaChaRng::from_seed(seed.0);

        Context {
            rng,
            seed,
            next_available_rest_port_number: Arc::clone(&self.next_available_rest_port_number),
            next_available_grpc_port_number: Arc::clone(&self.next_available_grpc_port_number),
            jormungandr: self.jormungandr().clone(),
            jcli: self.jcli().clone(),
        }
    }
}

impl<RNG: RngCore> Context<RNG> {
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
    pub fn seed(&self) -> &Seed {
        &self.seed
    }
}

impl Display for Seed {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        hex::encode(&self.0).fmt(f)
    }
}

impl FromStr for Seed {
    type Err = hex::FromHexError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes = hex::decode(s)?;

        let mut seed = Seed::zero();

        if bytes.len() != seed.0.len() {
            Err(hex::FromHexError::InvalidStringLength)
        } else {
            seed.0.copy_from_slice(&bytes);

            Ok(seed)
        }
    }
}
