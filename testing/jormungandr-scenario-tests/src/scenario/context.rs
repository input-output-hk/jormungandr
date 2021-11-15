use crate::scenario::ProgressBarMode;
use assert_fs::fixture::{ChildPath, PathChild};
use assert_fs::prelude::*;
use jormungandr_testing_utils::testing::jormungandr::TestingDirectory;
use jormungandr_testing_utils::testing::network::{Random, Seed};
use rand_chacha::ChaChaRng;
use rand_core::RngCore;
use std::path::{Path, PathBuf};

pub type ContextChaCha = Context<ChaChaRng>;

/// scenario context with all the details to setup the necessary port number
/// a pseudo random number generator (and its original seed).
#[derive(Clone)]
pub struct Context<RNG: RngCore + Sized = ChaChaRng> {
    rng: Random<RNG>,

    jormungandr: PathBuf,

    testing_directory: TestingDirectory,
    generate_documentation: bool,
    progress_bar_mode: ProgressBarMode,
    log_level: String,
}

impl Context<ChaChaRng> {
    pub fn new(
        seed: Seed,
        jormungandr: PathBuf,
        testing_directory: Option<PathBuf>,
        generate_documentation: bool,
        progress_bar_mode: ProgressBarMode,
        log_level: String,
    ) -> Self {
        let rng = Random::<ChaChaRng>::new(seed);

        Context {
            rng,
            jormungandr,
            testing_directory: testing_directory.into(),
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
