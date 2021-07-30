use assert_fs::fixture::{ChildPath, PathChild};
use assert_fs::prelude::*;
use assert_fs::TempDir;
use multiaddr::Multiaddr;
use rand_chacha::ChaChaRng;
use rand_core::RngCore;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::atomic::{self, AtomicU16};
use std::sync::Arc;

use crate::scenario::ProgressBarMode;
use jormungandr_testing_utils::testing::network_builder::{Random, Seed};

pub type ContextChaCha = Context<ChaChaRng>;

#[derive(Clone)]
enum TestingDirectory {
    Temp(Arc<TempDir>),
    User(PathBuf),
}

/// scenario context with all the details to setup the necessary port number
/// a pseudo random number generator (and its original seed).
#[derive(Clone)]
pub struct Context<RNG: RngCore + Sized = ChaChaRng> {
    rng: Random<RNG>,

    jormungandr: PathBuf,
    jcli: PathBuf,

    next_available_rest_port_number: Arc<AtomicU16>,
    next_available_grpc_port_number: Arc<AtomicU16>,

    testing_directory: TestingDirectory,
    generate_documentation: bool,
    progress_bar_mode: ProgressBarMode,
    log_level: String,
}

impl Context<ChaChaRng> {
    pub fn new(
        seed: Seed,
        jormungandr: PathBuf,
        jcli: PathBuf,
        testing_directory: Option<PathBuf>,
        generate_documentation: bool,
        progress_bar_mode: ProgressBarMode,
        log_level: String,
    ) -> Self {
        let rng = Random::<ChaChaRng>::new(seed);

        let testing_directory = if let Some(testing_directory) = testing_directory {
            TestingDirectory::User(testing_directory)
        } else {
            TestingDirectory::Temp(Arc::new(TempDir::new().unwrap()))
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
            log_level,
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
            jormungandr: self.jormungandr.clone(),
            jcli: self.jcli.clone(),
            testing_directory: self.testing_directory.clone(),
            generate_documentation: self.generate_documentation,
            progress_bar_mode: self.progress_bar_mode,
            log_level: self.log_level.clone(),
        }
    }

    pub(super) fn generate_documentation(&self) -> bool {
        self.generate_documentation
    }
}

impl<RNG: RngCore> Context<RNG> {
    pub fn jormungandr(&self) -> &Path {
        &self.jormungandr
    }

    pub fn child_directory(&self, path: impl AsRef<Path>) -> ChildPath {
        let child = self.child(path);
        child.create_dir_all().unwrap();
        child
    }

    pub fn child(&self, path: impl AsRef<Path>) -> ChildPath {
        self.testing_directory.child(path)
    }

    pub fn jcli(&self) -> &Path {
        &self.jcli
    }

    pub fn random(&mut self) -> &mut Random<RNG> {
        &mut self.rng
    }

    pub fn log_level(&self) -> String {
        self.log_level.clone()
    }

    pub fn generate_new_rest_listen_address(&mut self) -> SocketAddr {
        use std::net::{IpAddr, Ipv4Addr};

        let port_number = self.generate_new_unique_port();
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), port_number)
    }

    pub fn generate_new_unique_port(&mut self) -> u16 {
        self.next_available_rest_port_number
            .fetch_add(1, atomic::Ordering::SeqCst)
    }

    pub fn generate_new_grpc_public_address(&mut self) -> Multiaddr {
        use std::net::{IpAddr, Ipv4Addr};

        let port_number = self.generate_new_unique_port();
        let address = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));

        format!("/ip4/{}/tcp/{}", address, port_number)
            .parse()
            .unwrap()
    }

    /// retrieve the original seed of the pseudo random generator
    #[inline]
    pub fn seed(&self) -> &Seed {
        self.rng.seed()
    }

    pub fn progress_bar_mode(&self) -> ProgressBarMode {
        self.progress_bar_mode
    }
}

impl TestingDirectory {
    #[allow(dead_code)]
    pub fn path(&self) -> &Path {
        match self {
            TestingDirectory::User(path_buf) => path_buf,
            TestingDirectory::Temp(temp_dir) => temp_dir.path(),
        }
    }
}

impl PathChild for TestingDirectory {
    fn child<P>(&self, path: P) -> ChildPath
    where
        P: AsRef<Path>,
    {
        match self {
            TestingDirectory::User(dir_path) => ChildPath::new(dir_path.join(path)),
            TestingDirectory::Temp(temp_dir) => temp_dir.child(path),
        }
    }
}
