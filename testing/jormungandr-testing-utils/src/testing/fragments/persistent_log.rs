use bincode::Options;
use chain_core::property::Serialize;
use chain_impl_mockchain::fragment::Fragment;
use jormungandr_lib::interfaces::{
    load_persistent_fragments_logs_from_folder_path, PersistentFragmentLog,
};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

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

#[derive(custom_debug::Debug, thiserror::Error)]
pub enum Error {
    #[error("cannot serialize entry of persistent log")]
    Io(#[from] std::io::Error),
}
