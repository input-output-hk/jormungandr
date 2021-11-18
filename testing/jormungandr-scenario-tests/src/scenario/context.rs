use crate::scenario::ProgressBarMode;
use assert_fs::fixture::{ChildPath, PathChild};
use assert_fs::prelude::*;
use jormungandr_testing_utils::testing::jormungandr::TestingDirectory;
use std::path::{Path, PathBuf};

/// scenario context with all the details to setup the necessary port number
/// a pseudo random number generator (and its original seed).
#[derive(Clone)]
pub struct Context {
    jormungandr: PathBuf,
    testing_directory: TestingDirectory,
    generate_documentation: bool,
    progress_bar_mode: ProgressBarMode,
}

impl Context {
    pub fn new(
        jormungandr: PathBuf,
        testing_directory: Option<PathBuf>,
        generate_documentation: bool,
        progress_bar_mode: ProgressBarMode,
    ) -> Self {
        Context {
            jormungandr,
            testing_directory: testing_directory.into(),
            generate_documentation,
            progress_bar_mode,
        }
    }

    pub fn generate_documentation(&self) -> bool {
        self.generate_documentation
    }
}

impl Context {
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

    pub fn testing_directory(&self) -> &TestingDirectory {
        &self.testing_directory
    }

    pub fn progress_bar_mode(&self) -> ProgressBarMode {
        self.progress_bar_mode
    }
}
