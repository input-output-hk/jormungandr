use assert_fs::fixture::FixtureError;
use assert_fs::fixture::{ChildPath, PathChild};
use assert_fs::TempDir;
use std::path::{Path, PathBuf};
use thiserror::Error;

pub enum TestingDirectory {
    Temp(TempDir),
    User(PathBuf),
}

impl TestingDirectory {
    pub fn path(&self) -> &Path {
        match self {
            TestingDirectory::User(path_buf) => path_buf,
            TestingDirectory::Temp(temp_dir) => temp_dir.path(),
        }
    }

    pub fn new_temp() -> Result<Self, FixtureError> {
        Ok(Self::Temp(TempDir::new()?))
    }

    pub fn from_temp(temp_dir: TempDir) -> Self {
        Self::Temp(temp_dir)
    }

    pub fn into_persistent(self) -> Self {
        if let Self::Temp(temp_dir) = self {
            return Self::Temp(temp_dir.into_persistent());
        }
        self
    }
}

impl PathChild for TestingDirectory {
    fn child<P>(&self, path: P) -> ChildPath
    where
        P: AsRef<Path>,
    {
        match self {
            Self::User(dir_path) => ChildPath::new(dir_path.join(path)),
            Self::Temp(temp_dir) => temp_dir.child(path),
        }
    }
}

impl From<PathBuf> for TestingDirectory {
    fn from(path: PathBuf) -> Self {
        Self::User(path)
    }
}

impl From<Option<PathBuf>> for TestingDirectory {
    fn from(maybe_path: Option<PathBuf>) -> Self {
        if let Some(testing_directory) = maybe_path {
            testing_directory.into()
        } else {
            Default::default()
        }
    }
}

impl Clone for TestingDirectory {
    fn clone(&self) -> Self {
        match self {
            Self::User(dir_path) => dir_path.to_path_buf().into(),
            Self::Temp(_) => Default::default(),
        }
    }
}

impl Default for TestingDirectory {
    fn default() -> Self {
        Self::new_temp().unwrap()
    }
}

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Fixture(#[from] FixtureError),
}
