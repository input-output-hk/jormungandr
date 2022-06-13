#![allow(dead_code)]

use chain_core::{packer::Codec, property::Serialize};
use chain_impl_mockchain::fragment::Fragment;
use jormungandr_lib::interfaces::{
    load_persistent_fragments_logs_from_folder_path, PersistentFragmentLog,
};
use std::{
    fs::File,
    io::{BufWriter, Write},
    path::{Path, PathBuf},
};

pub struct PersistentLogViewer {
    dir: PathBuf,
}

impl PersistentLogViewer {
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    pub fn get_all(&self) -> Vec<Fragment> {
        load_persistent_fragments_logs_from_folder_path(&self.dir)
            .unwrap()
            .map(|x| x.unwrap().fragment)
            .collect()
    }

    pub fn get_bin(&self) -> Vec<Vec<u8>> {
        load_persistent_fragments_logs_from_folder_path(&self.dir)
            .unwrap()
            .map(|x| x.unwrap().fragment.serialize_as_vec().unwrap())
            .collect()
    }

    pub fn count(&self) -> usize {
        self.get_all().len()
    }
}
pub fn write_into_persistent_log<P: AsRef<Path>>(
    persistent_log: P,
    entries: Vec<PersistentFragmentLog>,
) -> Result<(), Error> {
    let mut output = BufWriter::with_capacity(128 * 1024, File::create(persistent_log.as_ref())?);

    for entry in entries {
        let mut codec = Codec::new(Vec::new());
        entry
            .serialize(&mut codec)
            .map_err(|_| Error::CannotSerializeEntry)?;
        output.write_all(codec.into_inner().as_slice())?;
    }
    Ok(())
}

#[derive(custom_debug::Debug, thiserror::Error)]
pub enum Error {
    #[error("cannot serialize entry of persistent log")]
    CannotSerializeEntry,
    #[error("cannot serialize entry of persistent log")]
    Io(#[from] std::io::Error),
}
