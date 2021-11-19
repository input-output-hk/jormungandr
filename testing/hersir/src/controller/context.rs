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
            log_level: session.log.to_string(),
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
    log_level: String,
}

impl Context {
    pub fn new(
        jormungandr: PathBuf,
        testing_directory: Option<PathBuf>,
        generate_documentation: bool,
        session_mode: SessionMode,
        log_level: String,
    ) -> Self {
        Context {
            jormungandr,
            testing_directory: testing_directory.into(),
            generate_documentation,
            session_mode,
            log_level
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

    pub fn session_mode(&self) -> SessionMode {
        self.session_mode
    }

    pub fn log_level(&self) -> String {
        self.log_level.to_string()
    }
}
