use rand_chacha::ChaChaRng;
use rand_core::RngCore;
use std::{
    net::SocketAddr,
    ops::Deref,
    path::{Path, PathBuf},
    sync::{
        atomic::{self, AtomicU16},
        Arc,
    },
};

use crate::scenario::ProgressBarMode;
use jormungandr_testing_utils::testing::network_builder::{Random, Seed};

pub type ContextChaCha = Context<ChaChaRng>;

#[derive(Clone)]
enum TestingDirectory {
    Temp(Arc<mktemp::Temp>),
    User(PathBuf),
}

/// scenario context with all the details to setup the necessary port number
/// a pseudo random number generator (and its original seed).
#[derive(Clone)]
pub struct Context<RNG: RngCore + Sized> {
    rng: Random<RNG>,

    jormungandr: bawawa::Command,
    jcli: bawawa::Command,

    next_available_rest_port_number: Arc<AtomicU16>,
    next_available_grpc_port_number: Arc<AtomicU16>,

    testing_directory: TestingDirectory,
    generate_documentation: bool,
    progress_bar_mode: ProgressBarMode,
}

impl Context<ChaChaRng> {
    pub fn new(
        seed: Seed,
        jormungandr: bawawa::Command,
        jcli: bawawa::Command,
        testing_directory: Option<PathBuf>,
        generate_documentation: bool,
        progress_bar_mode: ProgressBarMode,
    ) -> Self {
        let rng = Random::<ChaChaRng>::new(seed);

        let testing_directory = if let Some(testing_directory) = testing_directory {
            TestingDirectory::User(testing_directory)
        } else {
            TestingDirectory::Temp(Arc::new(mktemp::Temp::new_dir().unwrap()))
        };

        Context {
            rng,
            next_available_rest_port_number: Arc::new(AtomicU16::new(8_000)),
            next_available_grpc_port_number: Arc::new(AtomicU16::new(12_000)),
            jormungandr,
            jcli,
            testing_directory,
            generate_documentation,
            progress_bar_mode,
        }
    }

    /// derive the Context into a new context, seeding a new RNG from the original
    /// Context (so reproducibility is still available).
    pub fn derive(&mut self) -> Self {
        let seed = Seed::generate(self.rng.rng_mut());
        let rng = Random::<ChaChaRng>::new(seed);

        Context {
            rng,
            next_available_rest_port_number: Arc::clone(&self.next_available_rest_port_number),
            next_available_grpc_port_number: Arc::clone(&self.next_available_grpc_port_number),
            jormungandr: self.jormungandr().clone(),
            jcli: self.jcli().clone(),
            testing_directory: self.testing_directory.clone(),
            generate_documentation: self.generate_documentation,
            progress_bar_mode: self.progress_bar_mode,
        }
    }

    pub(super) fn working_directory(&self) -> &Path {
        &self.testing_directory
    }

    pub(super) fn generate_documentation(&self) -> bool {
        self.generate_documentation
    }
}

impl<RNG: RngCore> Context<RNG> {
    pub fn jormungandr(&self) -> &bawawa::Command {
        &self.jormungandr
    }

    pub fn jcli(&self) -> &bawawa::Command {
        &self.jcli
    }

    pub fn random(&mut self) -> &mut Random<RNG> {
        &mut self.rng
    }

    pub fn generate_new_rest_listen_address(&mut self) -> SocketAddr {
        use std::net::{IpAddr, Ipv4Addr};

        let port_number = self
            .next_available_rest_port_number
            .fetch_add(1, atomic::Ordering::SeqCst);
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), port_number)
    }

    pub fn generate_new_grpc_public_address(&mut self) -> poldercast::Address {
        use std::net::{IpAddr, Ipv4Addr};

        let port_number = self
            .next_available_grpc_port_number
            .fetch_add(1, atomic::Ordering::SeqCst);

        let address = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));

        format!("/ip4/{}/tcp/{}", address, port_number)
            .parse()
            .unwrap()
    }

    /// retrieve the original seed of the pseudo random generator
    #[inline]
    pub fn seed(&self) -> &Seed {
        &self.rng.seed()
    }

    pub fn progress_bar_mode(&self) -> ProgressBarMode {
        self.progress_bar_mode
    }
}

impl Deref for TestingDirectory {
    type Target = Path;
    fn deref(&self) -> &Self::Target {
        match self {
            TestingDirectory::User(ref path) => path.deref(),
            TestingDirectory::Temp(ref path) => path.deref(),
        }
    }
}
