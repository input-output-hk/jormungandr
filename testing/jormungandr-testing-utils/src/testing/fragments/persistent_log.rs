use chain_core::property::Serialize;
use chain_impl_mockchain::fragment::Fragment;
use jormungandr_lib::interfaces::load_fragments_from_folder_path;
use std::path::PathBuf;

pub struct PersistentLogViewer {
    dir: PathBuf,
}

impl PersistentLogViewer {
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    pub fn get_all(&self) -> Vec<Fragment> {
        load_fragments_from_folder_path(&self.dir)
            .unwrap()
            .map(|x| x.unwrap().fragment)
            .collect()
    }

    pub fn get_bin(&self) -> Vec<Vec<u8>> {
        load_fragments_from_folder_path(&self.dir)
            .unwrap()
            .map(|x| x.unwrap().fragment.serialize_as_vec().unwrap())
            .collect()
    }

    pub fn count(&self) -> usize {
        self.get_all().iter().count()
    }
}
