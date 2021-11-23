use crate::config::SessionMode;
use crate::config::SessionSettings;
use assert_fs::fixture::{ChildPath, PathChild};
use assert_fs::prelude::*;
use jormungandr_testing_utils::testing::jormungandr::TestingDirectory;
use std::path::{Path, PathBuf};

impl From<SessionSettings> for Context {
    fn from(session: SessionSettings) -> Self {
        Self {
            jormungandr: session
                .jormungandr
                .unwrap_or_else(|| Path::new("jormungandr").to_path_buf()),
            testing_directory: session.root.into(),
            generate_documentation: false,
            session_mode: session.mode,
        }
    }
}

impl Default for Context {
    fn default() -> Self {
        Self {
            jormungandr: Path::new("jormungandr").to_path_buf(),
            testing_directory: TestingDirectory::new_temp().unwrap(),
            generate_documentation: false,
            session_mode: SessionMode::Standard,
        }
    }
}

/// scenario context with all the details to setup the necessary port number
/// a pseudo random number generator (and its original seed).
#[derive(Clone)]
pub struct Context {
    jormungandr: PathBuf,
    testing_directory: TestingDirectory,
    generate_documentation: bool,
    session_mode: SessionMode,
}

impl Context {

    pub fn jormungandr(&self) -> &Path {
        &self.jormungandr
    }

    pub fn generate_documentation(&self) -> bool {
        self.generate_documentation
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

    pub fn session_mode(&self) -> SessionMode {
        self.session_mode
    }

}
