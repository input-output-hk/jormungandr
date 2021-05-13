use std::fs;
use std::io;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use crate::interfaces::FragmentDef;
use crate::time::SecondsSinceUnixEpoch;

use chain_impl_mockchain::fragment::Fragment;

use bincode::Options;
use serde::{Deserialize, Serialize};

#[derive(thiserror::Error, Debug)]
#[error("Couldn't deserialize entry {entry} in {file} due to: {cause}")]
pub struct DeserializeError {
    file: String,
    entry: usize,
    cause: bincode::Error,
}

/// Represents a persistent fragments log entry.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PersistentFragmentLog {
    /// the time this fragment was registered and accepted by the pool
    pub time: SecondsSinceUnixEpoch,
    /// full hex-encoded fragment body
    #[serde(with = "FragmentDef")]
    pub fragment: Fragment,
}

pub struct FileFragments {
    reader: BufReader<fs::File>,
    file_path: PathBuf,
}

pub struct FileFragmentsIterator {
    reader: BufReader<fs::File>,
    file_path: PathBuf,
    counter: usize,
}

impl FileFragments {
    pub fn from_path(
        file_path: PathBuf,
    ) -> std::io::Result<Box<dyn Iterator<Item = Result<PersistentFragmentLog, DeserializeError>>>>
    {
        let metadata = fs::metadata(file_path.clone())?;
        if metadata.len() == 0 {
            return Ok(Box::new(vec![].into_iter()));
        }
        let reader = BufReader::new(fs::File::open(file_path.clone())?);
        Ok(Box::new(Self { reader, file_path }.into_iter()))
    }
}

impl IntoIterator for FileFragments {
    type Item = Result<PersistentFragmentLog, DeserializeError>;
    type IntoIter = FileFragmentsIterator;

    fn into_iter(self) -> Self::IntoIter {
        FileFragmentsIterator {
            reader: self.reader,
            file_path: self.file_path,
            counter: 0,
        }
    }
}

impl Iterator for FileFragmentsIterator {
    type Item = Result<PersistentFragmentLog, DeserializeError>;

    fn next(&mut self) -> Option<Self::Item> {
        // EOF reached when buffer is empty.
        // Then we stop the iterator. File is guaranteed to be non-empty by construction, the check
        // cannot be done till buffer is filled, hence first time should be able to read at least something.
        if self.reader.buffer().is_empty() && self.counter != 0 {
            return None;
        }
        let codec = bincode::DefaultOptions::new()
            .with_fixint_encoding()
            .allow_trailing_bytes();

        let result = codec
            .deserialize_from(&mut self.reader)
            .map_err(|cause| DeserializeError {
                file: self.file_path.to_string_lossy().to_string(),
                entry: self.counter,
                cause,
            });
        self.counter += 1;
        Some(result)
    }
}

pub fn list_persistent_fragment_log_files_from_folder_path(
    folder: &Path,
) -> io::Result<impl Iterator<Item = PathBuf>> {
    let mut entries: Vec<_> = fs::read_dir(folder)?
        .filter_map(|entry| match entry {
            Ok(entry) => Some(folder.join(entry.path())),
            _ => None,
        })
        .collect();
    entries.sort();
    Ok(entries.into_iter())
}

pub fn read_persistent_fragment_logs_from_file_path(
    entries: impl Iterator<Item = PathBuf>,
) -> io::Result<impl Iterator<Item = Result<PersistentFragmentLog, DeserializeError>>> {
    let mut handles = Vec::new();
    for entry in entries {
        handles.push(FileFragments::from_path(entry)?);
    }
    Ok(handles.into_iter().flatten())
}

pub fn load_persistent_fragments_logs_from_folder_path(
    folder: &Path,
) -> io::Result<impl Iterator<Item = Result<PersistentFragmentLog, DeserializeError>>> {
    read_persistent_fragment_logs_from_file_path(
        list_persistent_fragment_log_files_from_folder_path(folder)?,
    )
}
