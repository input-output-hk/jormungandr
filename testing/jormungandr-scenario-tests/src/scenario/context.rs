use assert_fs::fixture::{ChildPath, PathChild};
use assert_fs::prelude::*;
use assert_fs::TempDir;
use rand_chacha::ChaChaRng;
use rand_core::RngCore;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::scenario::ProgressBarMode;
use jormungandr_testing_utils::testing::network::{Random, Seed};

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
